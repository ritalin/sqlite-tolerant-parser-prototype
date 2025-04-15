use sqlite_parser_proto::{Grammar, GrammarRuleMember, GrammarSymbol, RuleId, SymbolType};
use std::collections::{BTreeMap, HashMap};

use crate::{IdGenerator, config::ActionResolveConfig};

pub struct LalryBuilder {
    config: ActionResolveConfig,
}

impl LalryBuilder {
    pub fn new(grammar: &Grammar) -> Self {
        let config = ActionResolveConfig::new(&grammar);

        Self { config }
    }

    pub fn create_lalry_grammar(
        &self,
        grammar_rule: &Grammar,
        combination_rules: &HashMap<String, (String, Vec<String>)>,
        start_symbol: &str,
    ) -> lalry::Grammar<GrammarSymbol, String, RuleId> {
        let terminals = create_terminal_lookup(&grammar_rule.symbols);
        let rules = create_lalry_rules(grammar_rule, &terminals, &combination_rules);

        lalry::Grammar {
            rules,
            start: start_symbol.to_string(),
        }
    }

    pub fn convert_to_lalr<'a>(
        &'a self,
        grammar: &'a lalry::Grammar<GrammarSymbol, String, RuleId>,
    ) -> Result<lalry::LR1ParseTable<'a, GrammarSymbol, String, RuleId>, anyhow::Error> {
        let table = grammar.lalr1(&self.config).map_err(|conflict| {
            match conflict {
                lalry::LR1Conflict::ReduceReduce { state, token, r1: (name1, rhs1), r2: (name2, rhs2) } => {
                    anyhow::anyhow!("Reduce/Reduce conflict (token: {:?}, rule1: {} := {:?}, rule2: {} := {:?}, state/len: {}", token, name1, rhs1, name2, rhs2, state.items.len())
                }
                lalry::LR1Conflict::ShiftReduce { state, token, rule: (name, rhs) } => {
                    anyhow::anyhow!("Shift/Reduce conflict (token: {:?}), rule: {} := {:?}, state/len: {}", token, name, rhs, state.items.len())
                }
            }
        })?;

        Ok(table)
    }
}

fn create_terminal_lookup(sources: &[GrammarSymbol]) -> HashMap<String, GrammarSymbol> {
    let mut symbols = HashMap::new();

    for source in sources {
        match source.symbol_type {
            SymbolType::Terminal { .. } => {
                symbols
                    .entry(source.name.clone())
                    .or_insert_with(|| source.clone());
            }
            SymbolType::NonTerminal => {}
            SymbolType::MultiTerminal { .. } => {
                symbols
                    .entry(source.name.clone())
                    .or_insert_with(|| source.clone());
            }
        }
    }

    // FIXME: EOFとprogramをここで足す

    symbols
}

fn create_lalry_rules(
    grammar: &Grammar,
    terminals: &HashMap<String, GrammarSymbol>,
    combination_rules: &HashMap<String, (String, Vec<String>)>,
) -> BTreeMap<String, Vec<lalry::Rhs<GrammarSymbol, String, RuleId>>> {
    let mut rule_id_gen = IdGenerator::new(
        grammar
            .rules
            .iter()
            .map(|rule| rule.members.len() as u32)
            .sum::<u32>(),
    );

    let mut rules = grammar
        .rules
        .iter()
        .map(|rule| {
            let members = rule
                .members
                .iter()
                .flat_map(|member| {
                    create_symbols(&member, &terminals, &combination_rules, &mut rule_id_gen)
                })
                .map(|(id, symbols)| lalry::Rhs {
                    syms: symbols,
                    act: RuleId::new(id),
                })
                .collect::<Vec<_>>();
            (rule.lhs.clone(), members)
        })
        .collect::<BTreeMap<_, _>>();

    combination_rules
        .iter()
        .for_each(|(first, (new_name, followings))| {
            let id = rule_id_gen.id();
            let internal_syms = [first.clone()]
                .iter()
                .chain(followings)
                .map(|name| match terminals.get(name) {
                    Some(x) => lalry::Symbol::Terminal(x.clone()),
                    None => lalry::Symbol::Nonterminal(name.clone()),
                })
                .collect::<Vec<_>>();
            rules.insert(
                new_name.clone(),
                vec![lalry::Rhs {
                    syms: internal_syms,
                    act: RuleId::new(id),
                }],
            );
        });

    // FIXME: EOFとprogramをここで足す

    rules
}

fn create_symbols(
    rule: &GrammarRuleMember,
    terminals: &HashMap<String, GrammarSymbol>,
    combination_rules: &HashMap<String, (String, Vec<String>)>,
    id_gen: &mut IdGenerator,
) -> Vec<(u32, Vec<lalry::Symbol<GrammarSymbol, String>>)> {
    let mut symbols = vec![];
    let mut symbol = vec![];
    id_gen.set_current(rule.id);

    create_symbols_internal(
        &rule.sequences,
        terminals,
        combination_rules,
        id_gen,
        &mut symbol,
        &mut symbols,
    );

    symbols
}

fn create_symbols_internal(
    sequences: &[sqlite_parser_proto::Rhs],
    terminals: &HashMap<String, GrammarSymbol>,
    combination_rules: &HashMap<String, (String, Vec<String>)>,
    id_gen: &mut IdGenerator,
    current: &mut Vec<lalry::Symbol<GrammarSymbol, String>>,
    symbols: &mut Vec<(u32, Vec<lalry::Symbol<GrammarSymbol, String>>)>,
) {
    match sequences.get(0) {
        None => {
            symbols.push((id_gen.id(), current.clone()));
        }
        Some(seq) => {
            if let Some((new_rule, rest)) =
                try_reduce_combination(seq, &sequences[1..], combination_rules)
            {
                current.push(lalry::Symbol::Nonterminal(new_rule.name));
                create_symbols_internal(
                    &rest,
                    terminals,
                    combination_rules,
                    id_gen,
                    current,
                    symbols,
                );
                return;
            }

            match &seq.token {
                sqlite_parser_proto::Term::Symbol { name } if terminals.contains_key(name) => {
                    if let Some(grammar_sym) = terminals.get(name) {
                        match &grammar_sym.symbol_type {
                            SymbolType::Terminal { .. } | SymbolType::NonTerminal => {
                                current.push(lalry::Symbol::Terminal(grammar_sym.clone()));
                                create_symbols_internal(
                                    &sequences[1..],
                                    terminals,
                                    combination_rules,
                                    id_gen,
                                    current,
                                    symbols,
                                );
                            }
                            SymbolType::MultiTerminal { classes } => {
                                id_gen.flush();
                                for char_class in classes {
                                    if let Some(grammar_sym) = terminals.get(char_class) {
                                        let mut current = current.clone();
                                        current.push(lalry::Symbol::Terminal(grammar_sym.clone()));
                                        create_symbols_internal(
                                            &sequences[1..],
                                            terminals,
                                            combination_rules,
                                            id_gen,
                                            &mut current,
                                            symbols,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                sqlite_parser_proto::Term::Symbol { name } => {
                    current.push(lalry::Symbol::Nonterminal(name.clone()));
                    create_symbols_internal(
                        &sequences[1..],
                        terminals,
                        combination_rules,
                        id_gen,
                        current,
                        symbols,
                    );
                }
                sqlite_parser_proto::Term::CharClass { members } => {
                    id_gen.flush();
                    for term in members {
                        if let Some(grammar_sym) = terminals.get(term) {
                            let mut current = current.clone();
                            current.push(lalry::Symbol::Terminal(grammar_sym.clone()));
                            create_symbols_internal(
                                &sequences[1..],
                                terminals,
                                combination_rules,
                                id_gen,
                                &mut current,
                                symbols,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn try_reduce_combination<'a>(
    first: &'a sqlite_parser_proto::Rhs,
    sequences: &'a [sqlite_parser_proto::Rhs],
    combination_rules: &'a HashMap<String, (String, Vec<String>)>,
) -> Option<(GrammarSymbol, &'a [sqlite_parser_proto::Rhs])> {
    if let sqlite_parser_proto::Term::Symbol { name } = first.token.clone() {
        if let Some((new_name, followings)) = combination_rules.get(&name) {
            let matched =
                followings
                    .iter()
                    .zip(sequences)
                    .all(|(follow, seq)| match seq.token.clone() {
                        sqlite_parser_proto::Term::Symbol { name } if name == *follow => true,
                        _ => false,
                    });

            if matched {
                let new_rule = GrammarSymbol {
                    id: 0,
                    name: new_name.clone(),
                    symbol_type: SymbolType::NonTerminal,
                    precedence: None,
                };

                return Some((new_rule, &sequences[followings.len()..]));
            }
        }
    }

    None
}
