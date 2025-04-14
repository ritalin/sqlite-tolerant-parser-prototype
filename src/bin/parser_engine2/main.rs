use std::collections::{LinkedList, VecDeque};
use anyhow::bail;
use sqlite_parser_proto::{
    engine::{kinds as syntax_kind, resolve_parser_goto_state, resolve_parser_next_state, resolve_parser_accept_state}, 
    LookaheadTransition, SyntaxKind, TransitionEvent};

pub fn main() -> Result<(), anyhow::Error> {
    
    #[cfg(not(feature = "parser_generated"))]
    let mut tokens = VecDeque::<(SyntaxKind, Option<&str>)>::new();
    #[cfg(feature = "parser_generated")]
    let mut tokens = VecDeque::from_iter(
        vec![
            (syntax_kind::r#SELECT, None), 
            (syntax_kind::r#ID, Some("c")), (syntax_kind::r#DOT, Some(".")), (syntax_kind::r#ID, Some("code")), (syntax_kind::r#COMMA, Some(",")), 
            (syntax_kind::r#ID, Some("name")), 
            (syntax_kind::r#FROM, None), (syntax_kind::r#ID, Some("city")), (syntax_kind::r#ID, Some("c")), 
            (syntax_kind::r#SEMI, Some(";")), (syntax_kind::r#EOF, None)
        ].into_iter()
    );
    
    assert!(!tokens.is_empty(), "Need to generate pasing engine !");

    let mut state_stack = LinkedList::from([0]);
    loop {
        let current_state = *state_stack.back().unwrap();
        
        let lookahead = tokens.front();

        let event = match (fetch_parser_next_state(lookahead, current_state)?, lookahead) {
            (LookaheadTransition::Shift { next_state }, Some((kind, token))) => {
                let tag = kind.clone();
                let tk = token.map(String::from);

                let _ = tokens.pop_front();
                state_stack.push_back(next_state);
                TransitionEvent::Shift { syntax_kind: tag, next_state: next_state, current_state, input: tk }   
            }
            (LookaheadTransition::Reduce { pop_count, lhs }, Some((kind, _))) => {
                for _ in 0..pop_count {
                    state_stack.pop_back();
                }
                
                let peek = *state_stack.back().unwrap();
                eprintln!("Reduce:debug/kind: {}, state: {}, peek: {}, lhs: {}", kind.text, current_state, peek, lhs);
                let next_state = resolve_parser_goto_state(peek, lhs)?;
                
                state_stack.push_back(next_state);
                TransitionEvent::Reduce { next_state: next_state, current_state, pop_count: pop_count, syntax_kind: *kind }
            }
            (LookaheadTransition::Accept { last_kind, .. }, _) => {
                TransitionEvent::Accept { syntax_kind: last_kind, current_state }
            }
            _=> {
                bail!("Unexpected error (current_state: {current_state})");
            }
        };
        
        match event {
            TransitionEvent::Shift { syntax_kind, current_state, next_state, input } => {
                println!("Shift/kind: {}, state: {} -> {}, input: {:?}", syntax_kind.text, current_state, next_state, input);
            }
            TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count } => {
                println!("Reduce/kind: {}, state: {} -> {}, pop: {}", syntax_kind.text, current_state, next_state, pop_count);
            }
            TransitionEvent::Accept { syntax_kind, current_state } => {
                println!("Accept/kind: {}, state: {}", syntax_kind.text, current_state);
                break
            }
            TransitionEvent::Error { syntax_kind, failed_state, pop_count, candidate_syntax_kinds:_kinds } => {
                println!("Error/kind: {}, state: {}, pop: {}", syntax_kind.text, failed_state, pop_count);
            }
        }
    }

    Ok(())
}

fn fetch_parser_next_state(lookahead: Option<&(SyntaxKind, Option<&str>)>, current_state: usize) -> Result<LookaheadTransition, anyhow::Error> {
    match lookahead {
        Some(_) => {
            resolve_parser_next_state(current_state, &lookahead.unwrap().0)
        }
        None => {
            resolve_parser_accept_state(current_state)
        }
    }
}