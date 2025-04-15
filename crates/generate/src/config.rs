use sqlite_parser_proto::{GrammarRule, GrammarRuleMember, GrammarSymbol, Precedence, SymbolRef};
use std::collections::HashMap;

pub struct ActionResolveConfig {
    terminal_precedences: HashMap<u32, Precedence>,
    rule_precedences: HashMap<u32, Precedence>,
}

impl ActionResolveConfig {
    pub fn new(grammar: &sqlite_parser_proto::Grammar) -> Self {
        let terminal_precedences = grammar
            .symbols
            .iter()
            .filter_map(|GrammarSymbol { id, precedence, .. }| {
                precedence.as_ref().map(|p| (id.clone(), p.clone()))
            })
            .collect::<HashMap<_, _>>();
        let rule_precedences = grammar
            .rules
            .iter()
            .flat_map(|GrammarRule { members, .. }| members)
            .filter_map(|GrammarRuleMember { id, precedence, .. }| {
                precedence.as_ref().map(|p| (id.clone(), p.clone()))
            })
            .collect::<HashMap<_, _>>();

        Self {
            terminal_precedences,
            rule_precedences,
        }
    }

    fn find_precedence_by_id(id: u32, lookup: &HashMap<u32, Precedence>) -> Option<Precedence> {
        lookup.get(&id).cloned()
    }
}

impl ActionResolveConfig {
    pub fn find_rule_precedence<T, N, A>(&self, rhs: &lalry::Rhs<T, N, A>) -> Option<Precedence>
    where
        T: SymbolRef + std::fmt::Debug,
        N: std::fmt::Debug,
        A: SymbolRef + std::fmt::Debug,
    {
        if let Some(score) =
            ActionResolveConfig::find_precedence_by_id(rhs.act.id(), &self.rule_precedences)
        {
            return Some(score);
        }

        rhs.syms
            .iter()
            .rev()
            .filter_map(|member| match member {
                lalry::Symbol::Terminal(symbol) => ActionResolveConfig::find_precedence_by_id(
                    symbol.id(),
                    &self.terminal_precedences,
                ),
                lalry::Symbol::Nonterminal(_) => None,
            })
            .next()
    }
}

impl<'a, T, N, A> lalry::Config<'a, T, N, A> for ActionResolveConfig
where
    T: SymbolRef + std::fmt::Debug,
    N: std::fmt::Debug,
    A: SymbolRef + std::fmt::Debug,
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
        let prec_la = lookahead.and_then(|symbol| {
            ActionResolveConfig::find_precedence_by_id(symbol.id(), &self.terminal_precedences)
        });
        let prec_rhs = self.find_rule_precedence(&rhs);

        match (prec_rhs, prec_la) {
            (Some(prec_rhs), Some(prec_la)) => match prec_rhs.cmp(&prec_la) {
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Greater => true,
                _ => match prec_rhs {
                    Precedence::Left(_) => true,
                    Precedence::Right(_) => false,
                    Precedence::Noassoc => false,
                },
            },
            _ => true,
        }
    }

    fn priority_of(&self, rhs: &lalry::Rhs<T, N, A>, lookahead: Option<&T>) -> i32 {
        if let Some(prec_rhs) = self.find_rule_precedence(&rhs) {
            return prec_rhs.score();
        }
        if let Some(prec_la) = lookahead.and_then(|symbol| {
            ActionResolveConfig::find_precedence_by_id(symbol.id(), &self.terminal_precedences)
        }) {
            return prec_la.score();
        }

        0
    }
}
