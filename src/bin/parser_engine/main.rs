mod engine;

use std::collections::{HashMap, LinkedList, VecDeque};
use engine::{eof_transition, init_goto_transition_table, init_lookahead_translations, syntax_kind, LookaheadTransition, SyntaxKind, TransitionEvent};

pub fn main() {
    let lookahead_transitions = init_lookahead_translations();
    let eof_transition = eof_transition();
    let goto_transitions = init_goto_transition_table();

    let mut tokens = VecDeque::from_iter(
        vec![
            (syntax_kind::r#SELECT, None), 
            (syntax_kind::r#ID, Some("c")), (syntax_kind::r#DOT, Some(".")), (syntax_kind::r#ID, Some("code")), (syntax_kind::r#COMMA, Some(",")), 
            (syntax_kind::r#ID, Some("name")), 
            (syntax_kind::r#FROM, None), (syntax_kind::r#ID, Some("city")), (syntax_kind::r#ID, Some("c")), 
            (syntax_kind::r#SEMI, Some(";")), (syntax_kind::r#EOF, None)
        ].into_iter()
    );

    let mut state_stack = LinkedList::from([0]);
    // let mut node_stack = LinkedList::new();

    loop {
        let current_state = *state_stack.back().unwrap();
        let lookahead = tokens.front();

        let transition = fetch_transition(lookahead, current_state, lookahead_transitions, eof_transition);
        let event = match (transition, lookahead) {
            (Some(LookaheadTransition::Shift { next_state }), Some((tag, tk))) => {
                let tag = tag.clone();
                let tk = tk.map(String::from);

                let _ = tokens.pop_front();
                state_stack.push_back(next_state);
                TransitionEvent::Shift { next_state: next_state, current_state, syntax_kind: tag, input: tk }
            }
            (Some(LookaheadTransition::Reduce { pop_count, lhs }), Some((tag, _))) => {
                for _ in 0..pop_count {
                    state_stack.pop_back();
                }
                
                let peek = *state_stack.back().unwrap();
                eprintln!("Reduce:debug/kind: {}, state: {}, peek: {}, lhs: {}", tag.text, current_state, peek, lhs);
                let goto_transition = goto_transitions[peek].as_ref().expect("Missing reduce state");

                let next_state = goto_transition.get(&lhs).expect("Missing goto");
                
                state_stack.push_back(*next_state);
                TransitionEvent::Reduce { next_state: *next_state, current_state, pop_count: pop_count, syntax_kind: tag.clone() }
            }
            (Some(LookaheadTransition::Accept { last_state, last_kind }), None) if last_state == current_state => {
                TransitionEvent::Accept { syntax_kind: last_kind, current_state }
            }
            _ => {
                panic!("Parse error at {:?}, state: {}", lookahead, current_state);
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
}

fn fetch_transition(lookahead: Option<&(SyntaxKind, Option<&str>)>, current_state: usize, lookahead_transitions: &[HashMap<u32, LookaheadTransition>], (eof_state, _): (usize, usize)) -> Option<LookaheadTransition> {
    let Some((kind, _)) = lookahead else {
        return Some(LookaheadTransition::Accept{ last_state: eof_state, last_kind: syntax_kind::r#program });
    };

    lookahead_transitions[current_state].get(&kind.id).cloned()
}