use std::collections::HashMap;
use proc_macro2::TokenStream;
use sqlite_parser_proto::{GrammarSymbol, RuleId};
use quote::quote;

use crate::{tokens_to_string, with_indent};

pub fn export_parser_state(machine: &lalry::LR1ParseTable<'_, GrammarSymbol, String, RuleId>, start_symbol: &str, symbols: &[GrammarSymbol]) -> String {
    let lookup = HashMap::<String, u32>::from_iter(symbols.iter().map(|x| (x.name.clone(), x.id)));

    let lookahead_state = export_lookahead_parser_state(&machine.states, &lookup);
    let goto_state = export_goto_parser_state(&machine.states, &lookup);
    let eof_state = export_eof_parser_state(&machine.states, lookup.get(start_symbol));

    let tokens = quote! {
        use phf::phf_map;
        type LATransition = LookaheadTransition;
        #lookahead_state
        #goto_state
        #eof_state
    };

    tokens.to_string()
}

fn export_lookahead_parser_state(states: &[lalry::LR1State<'_, GrammarSymbol, String, RuleId>], lookup: &HashMap<String, u32>) -> TokenStream {
    let lookahead_states: TokenStream = states.iter()
        .map(|state| {
            let rules: TokenStream = state.lookahead.iter()
                .filter_map(|(la, action)| {
                    export_lookahead_state_transition_member(la.id, action, &lookup)
                })
                .collect()
            ;

            quote!{
                phf_map! { #rules },
            }
        })
        .collect()
    ;

    quote! {
        pub static LA_TRANSITION_TABLE: &[phf::Map<u32, usize>] = &[
            #lookahead_states
        ];
    }
}

fn export_goto_parser_state(states: &[lalry::LR1State<'_, GrammarSymbol, String, RuleId>], lookup: &HashMap<String, u32>) -> TokenStream {
    let goto_states: TokenStream = states.iter()
        .flat_map(|state| {
            let members: TokenStream = state.goto.iter()
                .map(|(symbol, next_state)| export_goto_transition_member(symbol, *next_state, lookup))
                .collect()
            ;

            match state.goto.is_empty() {
                false => {
                    quote! {
                        Some(phf_map! {
                            #members
                        }),
                    }
                }
                true => {
                    quote! {
                        None,
                    }
                }
            }
        })
        .collect()
    ;

    quote! {
        pub static LA_TRANSITION_TABLE: &[Option<phf::Map<u32, usize>>] = &[
            #goto_states
        ];
    }
}

fn export_eof_parser_state(states: &[lalry::LR1State<'_, GrammarSymbol, String, RuleId>], start_kind: Option<&u32>) -> TokenStream {
    let eof_state: Option<usize> = states.iter().enumerate()
        .flat_map(|(i, state)| state.eof.as_ref().map(|_| i))
        .next()
    ;

    match (eof_state, start_kind) {
        (Some(state), Some(kind)) => {
            quote! {
                pub static EOF_TRANSITION_STATE: usize = #state;
                pub static EOF_TRANSITION_KIND: u32 = #kind;
            }            
        }
        _ => {
            panic!("Unresolved EOF state (start_kind: {start_kind:?})")
        }
    }
}

fn export_lookahead_state_transition_member(la_id: u32, action: &lalry::LRAction<'_, GrammarSymbol, String, RuleId>, lookup: &HashMap<String, u32>) -> Option<TokenStream> {
    match action {
        lalry::LRAction::Reduce(lhs, rhs) => {
            let pop_count = rhs.syms.len();
            let lhs_id = lookup.get(*lhs).expect("Mismatch symbol id");

            let rule = quote! {
                #la_id => LATransition::Reduce { pop_count: #pop_count, lhs: #lhs_id },
            };
            Some(rule)
        }
        lalry::LRAction::Shift(next_state) => {
            let rule = quote! {
                #la_id => LATransition::Shift { next_state: #next_state },
            };
            Some(rule)
        }
        lalry::LRAction::Accept => {
            None
        }
    }
}

fn export_goto_transition_member(symbol: &String, next_state: usize, lookup: &HashMap<String, u32>) -> TokenStream {
    let symbol_id = lookup.get(symbol).expect("Mismatch symbol id");

    quote! {
        #symbol_id => #next_state,
    }
}

fn create_map_comment(state: usize, depth: usize) -> String {
    with_indent(&format!("// state: #{state}"), depth)
}

pub fn export_parser_state_pretty(machine: &lalry::LR1ParseTable<'_, GrammarSymbol, String, RuleId>, start_symbol: &str, symbols: &[GrammarSymbol]) -> String {
    let lookup = HashMap::<String, u32>::from_iter(symbols.iter().map(|x| (x.name.clone(), x.id)));

    let iter = vec![
        "use phf::phf_map;".to_string(),
        "type LATransition = LookaheadTransition;".to_string(),
    ].into_iter();

    iter.chain(export_parser_lookahead_state_pretty(&machine.states, &lookup))
        .chain(export_parser_goto_state_pretty(&machine.states, &lookup))
        .chain(export_eof_parser_state_pretty(&machine.states, lookup.get(start_symbol)))
        .collect::<Vec<_>>().join("\n")
}

fn export_parser_lookahead_state_pretty(states: &[lalry::LR1State<'_, GrammarSymbol, String, RuleId>], lookup: &HashMap<String, u32>) -> impl Iterator<Item = String> {
    let lookahead_states = states.iter().enumerate()
        .flat_map(|(i, state)| {
            let members = state.lookahead.iter()
                .filter_map(|(la, action)| export_lookahead_state_transition_member(la.id, action, &lookup))
                .map(|tokens| tokens_to_string(tokens, 2))
            ;

            let iter = std::iter::empty();
            iter.chain(vec![
                    create_map_comment(i, 1),
                    with_indent("phf_map! {", 1),
                ])
                .chain(members)
                .chain(vec![with_indent("},", 1)])
        })
        .collect::<Vec<_>>()
    ;

    let iter = vec![
        "pub static LA_TRANSITION_TABLE: &[phf::Map<u32, LATransition>] = &[".to_string(),
    ].into_iter();

    iter.chain(lookahead_states)
        .chain(vec!["];".into()])
}

fn export_parser_goto_state_pretty(states: &[lalry::LR1State<'_, GrammarSymbol, String, RuleId>], lookup: &HashMap<String, u32>) -> impl Iterator<Item = String> {
    let goto_states = states.iter().enumerate()
        .flat_map(|(i, state)| {
            let members = state.goto.iter()
                .map(|(symbol, next_state)| export_goto_transition_member(symbol, *next_state, lookup))
                .map(|tokens| tokens_to_string(tokens, 2))
                .collect::<Vec<_>>()
            ;
            create_goto_map(i, members)
        })
        .collect::<Vec<_>>()
    ;

    let iter = vec![
        "pub static GOTO_TRANSITION_TABLE: &[Option<phf::Map<u32, usize>>] = &[".to_string(),
    ].into_iter();

    iter.chain(goto_states)
        .chain(vec!["];".into()])
}

fn create_goto_map(state: usize, members: Vec<String>) -> Vec<String> {
    let iter = vec![create_map_comment(state, 1)].into_iter();
    
    match members.is_empty() {
        false => {
            iter
                .chain(vec![with_indent("Some(phf_map! {", 1)])
                .chain(members)
                .chain(vec![with_indent("}),", 1)])
                .collect()
        }
        true => {
            iter.chain(vec![with_indent("None,", 1)])
                .collect()
        }
    }
}

fn export_eof_parser_state_pretty(states: &[lalry::LR1State<'_, GrammarSymbol, String, RuleId>], start_kind: Option<&u32>) -> Vec<String> {
    let eof_state: Option<usize> = states.iter().enumerate()
    .flat_map(|(i, state)| state.eof.as_ref().map(|_| i))
    .next()
;

match (eof_state, start_kind) {
    (Some(state), Some(kind)) => {
        vec![
            tokens_to_string(quote! { pub static EOF_TRANSITION_STATE: usize = #state; }, 0),
            tokens_to_string(quote! { pub static EOF_TRANSITION_KIND: u32 = #kind; }, 0),
        ]        
    }
    _ => {
        panic!("Unresolved EOF state (start_kind: {start_kind:?})")
    }
}
}