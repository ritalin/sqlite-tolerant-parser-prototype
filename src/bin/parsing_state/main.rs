use std::{collections::{BTreeMap, HashMap, LinkedList}, fmt::Debug};

use sqlite_parser_proto::{Grammar, GrammarRule, GrammarRuleMember, GrammarSymbol, Precedence, RuleId, SymbolRef, SymbolType};

pub fn main() -> Result<(), anyhow::Error> {
    let gramer_rule = serde_json::from_str::<Grammar>(include_str!("../../../build/grammar.json"))?;

    let terminals = create_terminal_lookup(&gramer_rule.symbols);

    let combination_rules = HashMap::<String, (String, Vec<String>)>::from_iter(vec![
        ("IS".into(), ("ISNOT".into(), vec!["NOT".into(), ]))
    ]);

    let mut id_gen = IdGenerator::new(gramer_rule.rules.iter().map(|rule| rule.members.len()).sum::<usize>());

    let mut rules = gramer_rule.rules.iter()
        .map(|rule| {
            let members = rule.members.iter()
                .flat_map(|member| create_symbols(&member, &terminals, &combination_rules, &mut id_gen))
                .map(|(id, symbols)| lalry::Rhs { syms: symbols, act: RuleId::new(id) })
                .collect::<Vec<_>>()
            ;
            (rule.lhs.clone(), members)
        })
        .collect::<BTreeMap<_, _>>()
    ;

    combination_rules.iter()
        .for_each(|(first, (new_name, followings))| {
            let id = rules.len();
            let internal_syms = [first.clone()].iter().chain(followings)
                .map(|name| {
                    match terminals.get(name) {
                        Some(x) => {
                            lalry::Symbol::Terminal(x.clone())
                        }
                        None => {
                            lalry::Symbol::Nonterminal(name.clone())
                        }
                    }
                })
                .collect::<Vec<_>>()
            ;
            rules.entry(new_name.clone()).or_insert_with(|| vec![lalry::Rhs{ syms: internal_syms, act: RuleId::new(id) }]);
        })
    ;

    let start_symbol = "program".to_string();
    let eof_symbol = GrammarSymbol { id: terminals.len() + 1, name: "EOF".into(), symbol_type: SymbolType::NonTerminal, precedence: None };
    rules.insert(start_symbol.clone(), vec![
        lalry::Rhs{ syms: vec![lalry::Symbol::Nonterminal(gramer_rule.start.clone()) , lalry::Symbol::Terminal(eof_symbol)], act: RuleId { id: rules.len() } }
    ]);

    let grammar = lalry::Grammar {
        rules,
        start: start_symbol.clone(),
    };

    // dump_grammar(&grammar);

    let config = ActionResolveConfig::new(&gramer_rule);

    // let machine = grammar.lr0_state_machine();

    // for (i, (items, transitions)) in machine.states.iter().enumerate() {
    //     println!("#{:<4}", i);
    //     println!("items: (len: {})", items.items.len());
    //     for (j, lalry::Item { lhs, rhs, pos }) in items.items.iter().enumerate() {
    //         println!("  ##{:<3} lhs: {:?}, rhs: {:?}, pos: {}", j, lhs, rhs, pos);
    //     }
    //     println!("transitions: (len: {})", transitions.len());
    //     for (j, transition) in transitions.iter().enumerate() {
    //         println!("  ##{:<3} next: {:?}", j, transition);
    //     }
    //     println!("--------------------------------------------------------------------------------")
    // }

    // let extended = machine.extended_grammar();
    // let first_sets = extended.first_sets();
    // let follow_sets = extended.follow_sets(first_sets);

    // for (i, (&&(s2, l2), &(ref follow, eof))) in follow_sets.iter().enumerate() {
    //     println!("#{:<4} s2: {}, l2: {}, eof: {}", i, s2, l2, eof);
    //     for follow in follow {
    //         println!("{follow}");
    //     }
    //     println!("--------------------------------------------------------------------------------")
    // }

    let table = grammar.lalr1(&config).map_err(|conflict| {
        match conflict {
            lalry::LR1Conflict::ReduceReduce { state, token, r1: (name1, rhs1), r2: (name2, rhs2) } => {
                anyhow::anyhow!("Reduce/Reduce conflict (token: {:?}, rule1: {} := {:?}, rule2: {} := {:?}, state/len: {}", token, name1, rhs1, name2, rhs2, state.items.len())
            }
            lalry::LR1Conflict::ShiftReduce { state, token, rule: (name, rhs) } => {
                anyhow::anyhow!("Shift/Reduce conflict (token: {:?}), rule: {} := {:?}, state/len: {}", token, name, rhs, state.items.len())
            }
        }
    })?;

    dump_parse_table(&table, &gramer_rule.symbols);
    // test_parse(&table);

    Ok(())
}

#[allow(unused)]
fn dump_grammar(grammar: &lalry::Grammar<GrammarSymbol, String, RuleId>) {
    for (i, (lhs, rule)) in grammar.rules.iter().enumerate() {
        println!("#{:<5} lhs: {}", i, lhs);
        for (j, rhs) in rule.iter().enumerate() {
            println!("  ##{:<5} {:?}", j, rhs);
        }
        println!("--------------------------------------------------------------------------------");
    }
}

#[allow(unused)]
fn dump_parse_table<'a>(table: &lalry::LR1ParseTable<'a, GrammarSymbol, String, RuleId>, symbols: &[GrammarSymbol]) {
    let lookup = symbols.iter().map(|x| (x.name.clone(), x.id)).collect::<HashMap<_, _>>();

    for (i, state) in table.states.iter().enumerate() {
        println!("#{:<5} EOF: {:?}", i, state.eof);
        println!("#{:<5} Goto tables/len: {}", i, state.goto.len());
        for (j, (symbol, dest)) in state.goto.iter().enumerate() {
            println!("  ##{:<6} {} ({:?}) -> {}", j, symbol, lookup.get(*symbol), dest);
        }

        println!("--------------------------------------------------------------------------------");
        println!("#{:<5} Lookahead tables/len: {}", i, state.lookahead.len());
        for (j, (k, action)) in state.lookahead.iter().enumerate() {
            println!("  ##{:<6} {:?} -> {:?}", j, k, action);
        }

        println!("\n================================================================================\n");
    }
}

#[allow(unused)]
fn test_parse<'a>(table: &lalry::LR1ParseTable<'a, GrammarSymbol, String, RuleId>) 
{
    let mut tokens = LinkedList::from_iter(
        vec![
            ("SELECT", None), ("ID", Some("c")), ("DOT", Some(".")), ("ID", Some("code")), ("COMMA", Some(",")), ("ID", Some("name")), ("FROM", None), ("ID", Some("city")), ("ID", Some("c")), ("SEMI", Some(";")), ("EOF", None)
        ].into_iter()
    );
    let mut state_stack = LinkedList::from([0]);
    let mut value_stack = LinkedList::new();

    loop {
        let current_state = *state_stack.back().unwrap();
        let lookahead = tokens.front();

        let action = match lookahead {
            Some((tag, tk)) => {
                match table.states[current_state].lookahead.iter().find(|(symbol, _)| symbol.name == *tag) {
                    Some((_, action)) => Some(action),
                    None => None,
                }
            }
            None => {
                table.states[current_state].eof.as_ref()
            }
        };

        match action {
            Some(lalry::LRAction::Shift(next_state)) => {
                let (tag, tk) = tokens.pop_front().unwrap();
                state_stack.push_back(*next_state);
                value_stack.push_back(tk.unwrap_or(tag).to_string());
                println!("** Shift/state: {} -> {}, eat: {} ({:?})", current_state, next_state, tag, tk);
            }
            Some(lalry::LRAction::Reduce(lhs, rhs)) => {
                for _ in 0..rhs.syms.len() {
                    state_stack.pop_back();
                }

                let values = value_stack.split_off(value_stack.len() - rhs.syms.iter().len()).iter()
                    .map(|v| format!("Node(`{v}`)"))
                    .collect::<Vec<_>>()
                ;
                value_stack.push_back(format!("Node({})", values.join(",")));

                let peek = *state_stack.back().unwrap();
                let next_state = table.states[peek].goto.get(lhs).expect("Missing goto");
                state_stack.push_back(*next_state);
                println!("** Reduce/state: {}, name: {}, rhs/len: {}, goto: {} -> {}", current_state, lhs, rhs.syms.len(), peek, next_state);
            }
            Some(lalry::LRAction::Accept) => {
                let result = value_stack.pop_back();
                println!("Parse succeeded: {:?}", value_stack);
                break;
            }
            None => {
                panic!("Parse error at {:?}", lookahead);
            }
        }
    }
}

// --------------------------------------------------------------------------------

fn create_terminal_lookup(sources: &[GrammarSymbol]) -> HashMap<String, GrammarSymbol> {
    let mut symbols = HashMap::new();

    for source in sources {
        match source.symbol_type {
            SymbolType::Terminal { .. } => {
                symbols.entry(source.name.clone()).or_insert_with(|| source.clone());
            }
            SymbolType::NonTerminal => {
            }
            SymbolType::MultiTerminal { .. } => {
                symbols.entry(source.name.clone()).or_insert_with(|| source.clone());
            }
        }
    }

    symbols
}

struct IdGenerator {
    stack: LinkedList<usize>,
    next: usize,
}

impl IdGenerator {
    pub fn new(next_id: usize) -> Self {
        Self {
            stack: LinkedList::new(),
            next: next_id,
        }
    }

    pub fn set_current(&mut self, id: usize) {
        self.stack.push_back(id);
    }
    pub fn flush(&mut self) {
        self.stack.clear();
    }
    pub fn id(&mut self) -> usize {
        match self.stack.pop_back() {
            Some(id) => id,
            None => {
                let id = self.next;
                self.next += 1;
                id
            }
        }
    }
}

fn create_symbols(rule: &GrammarRuleMember, terminals: &HashMap<String, GrammarSymbol>, combination_rules: &HashMap<String, (String, Vec<String>)>, id_gen: &mut IdGenerator) -> Vec<(usize, Vec<lalry::Symbol<GrammarSymbol, String>>)> {
    let mut symbols = vec![];
    let mut symbol = vec![];
    id_gen.set_current(rule.id);

    create_symbols_internal(&rule.sequences, terminals, combination_rules, id_gen, &mut symbol, &mut symbols);

    symbols
}

fn create_symbols_internal(sequences: &[sqlite_parser_proto::Rhs], terminals: &HashMap<String, GrammarSymbol>, combination_rules: &HashMap<String, (String, Vec<String>)>, id_gen: &mut IdGenerator, current: &mut Vec<lalry::Symbol<GrammarSymbol, String>>, symbols: &mut Vec<(usize, Vec<lalry::Symbol<GrammarSymbol, String>>)>) {
    match sequences.get(0) {
        None => {
            symbols.push((id_gen.id(), current.clone()));
        }
        Some(seq) => {
            if let Some((new_rule, rest)) = try_reduce_combination(seq, &sequences[1..], combination_rules) {
                current.push(lalry::Symbol::Nonterminal(new_rule.name));
                create_symbols_internal(&rest, terminals, combination_rules, id_gen, current, symbols);
                return;
            }

            match &seq.token {
                sqlite_parser_proto::Term::Symbol { name } if terminals.contains_key(name) => {
                    if let Some(grammar_sym)  = terminals.get(name) {
                        match &grammar_sym.symbol_type {
                            SymbolType::Terminal { .. } | SymbolType::NonTerminal => {
                                current.push(lalry::Symbol::Terminal(grammar_sym.clone()));
                                create_symbols_internal(&sequences[1..], terminals, combination_rules, id_gen, current, symbols);
                            }
                            SymbolType::MultiTerminal { classes } => {
                                id_gen.flush();
                                for char_class in classes {
                                    if let Some(grammar_sym) = terminals.get(char_class) {
                                        let mut current = current.clone();
                                        current.push(lalry::Symbol::Terminal(grammar_sym.clone()));
                                        create_symbols_internal(&sequences[1..], terminals, combination_rules, id_gen, &mut current, symbols);
                                    }
                                }
                            }
                        }
                    }
                }
                sqlite_parser_proto::Term::Symbol { name } => {
                    current.push(lalry::Symbol::Nonterminal(name.clone()));
                    create_symbols_internal(&sequences[1..], terminals, combination_rules, id_gen, current, symbols);
                }
                sqlite_parser_proto::Term::CharClass { members } => {
                    id_gen.flush();
                    for term in members {
                        if let Some(grammar_sym)  = terminals.get(term) {
                            let mut current = current.clone();
                            current.push(lalry::Symbol::Terminal(grammar_sym.clone()));
                            create_symbols_internal(&sequences[1..], terminals, combination_rules, id_gen, &mut current, symbols);
                        }
                    }
                }
            }
        }
    }
}

fn try_reduce_combination<'a>(first: &'a sqlite_parser_proto::Rhs, sequences: &'a [sqlite_parser_proto::Rhs], combination_rules: &'a HashMap<String, (String, Vec<String>)>) -> Option<(GrammarSymbol, &'a [sqlite_parser_proto::Rhs])> {
    if let sqlite_parser_proto::Term::Symbol{ name } = first.token.clone() {
        if let Some((new_name, followings)) = combination_rules.get(&name) {
            let matched = followings.iter().zip(sequences)
            .all(|(follow, seq)| match seq.token.clone() {
                sqlite_parser_proto::Term::Symbol { name } if name == *follow => true,
                _ => false
            });

            if matched {
                let new_rule = GrammarSymbol { 
                    id: 0, 
                    name: new_name.clone(), 
                    symbol_type: SymbolType::NonTerminal, 
                    precedence: None 
                };

                return Some((new_rule, &sequences[followings.len()..]));
            }
        }
    }

    None
}

struct ActionResolveConfig {
    terminal_precedences: HashMap<usize, Precedence>,
    rule_precedences: HashMap<usize, Precedence>
}

impl ActionResolveConfig {
    pub fn new(grammar: &sqlite_parser_proto::Grammar) -> Self {
        let terminal_precedences = grammar.symbols.iter()
            .filter_map(|GrammarSymbol{ id, precedence, .. }| {
                precedence.as_ref().map(|p| (id.clone(), p.clone()))
            })
            .collect::<HashMap<_, _>>()
        ;
        let rule_precedences = grammar.rules.iter()
            .flat_map(|GrammarRule{ members, .. }| members)
            .filter_map(|GrammarRuleMember { id, precedence, .. }| {
                precedence.as_ref().map(|p| (id.clone(), p.clone()))
            })
            .collect::<HashMap<_, _>>()
        ;

        Self { terminal_precedences, rule_precedences }
    }

    fn find_precedence_by_id(id: usize, lookup: &HashMap<usize, Precedence>) -> Option<Precedence> {
        lookup.get(&id).cloned()
    }
}

impl ActionResolveConfig {
    pub fn find_rule_precedence<T, N, A>(&self, rhs: &lalry::Rhs<T, N, A>) -> Option<Precedence> 
        where T: SymbolRef + Debug, N: Debug, A: SymbolRef + Debug     
    {
        if let Some(score) = ActionResolveConfig::find_precedence_by_id(rhs.act.id(), &self.rule_precedences) {
            return Some(score);
        }

        rhs.syms.iter().rev()
            .filter_map(|member| match member {
                lalry::Symbol::Terminal(symbol) => {
                    ActionResolveConfig::find_precedence_by_id(symbol.id(), &self.terminal_precedences)
                }
                lalry::Symbol::Nonterminal(_) => None
            })
            .next()
    }

}

impl<'a, T, N, A> lalry::Config<'a, T, N, A> for ActionResolveConfig 
    where T: SymbolRef + Debug, N: Debug, A: SymbolRef + Debug
{
    fn resolve_shift_reduce_conflict_in_favor_of_shift(&self) -> bool {
        true
    }

    fn warn_on_resolved_conflicts(&self) -> bool {
        // println!("`warn_on_resolved_conflicts` called");
        false
    }

    fn on_resolved_conflict(&self, _conflict: lalry::LR1ResolvedConflict<'a, T, N, A>) {
        // println!("`on_resolved_conflict` called");
    }

    fn reduce_on(&self, rhs: &lalry::Rhs<T, N, A>, lookahead: Option<&T>) -> bool {
        let prec_la = lookahead.and_then(|symbol| ActionResolveConfig::find_precedence_by_id(symbol.id(), &self.terminal_precedences));
        let prec_rhs = self.find_rule_precedence(&rhs);

        match (prec_rhs, prec_la) {
            (Some(prec_rhs), Some(prec_la)) => { 
                match prec_rhs.cmp(&prec_la) {
                    std::cmp::Ordering::Less => false,
                    std::cmp::Ordering::Greater => true,
                    _ => {
                        match prec_rhs {
                            Precedence::Left(_) => true,
                            Precedence::Right(_) => false,
                            Precedence::Noassoc => false,
                        }
                    }
                }
            }
            _ => true
        }
    }

    fn priority_of(&self, rhs: &lalry::Rhs<T, N, A>, lookahead: Option<&T>) -> i32 {
        if let Some(prec_rhs) = self.find_rule_precedence(&rhs) {
            return prec_rhs.score();
        }
        if let Some(prec_la) = lookahead.and_then(|symbol| ActionResolveConfig::find_precedence_by_id(symbol.id(), &self.terminal_precedences)) {
            return prec_la.score();
        }

        0
    }
}
