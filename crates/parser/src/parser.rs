use std::collections::{LinkedList, HashMap};
use anyhow::bail;
use cstree::{build::NodeCache, green::{GreenNode, GreenToken}, util::NodeOrToken};
use scanner::{Scanner, Token};
use sqlite_parser_proto::{LookaheadTransition, SyntaxKind, TransitionEvent, engine::kinds as syntax_kind};
use crate::{InternCache, Language, SyntaxTree, Annotation};


pub struct Parser {
    language: Language,
}

type NodeElement = NodeOrToken::<GreenNode, GreenToken>;

impl Parser {
    pub fn new() -> Self {
        Self {
            language: Default::default(),
        }
    }

    pub fn parse(&self, source: &str) -> Result<SyntaxTree, anyhow::Error> {
        let mut state_stack = LinkedList::from([0]);
        let mut element_stack = LinkedList::new();
        let mut intern_cache = InternCache::new();
        let mut cache = NodeCache::with_interner(&mut intern_cache);
        let mut node_annotations = HashMap::<NodeElement, (Annotation, AnnotationStatus)>::new();

        let mut scanner = Scanner::create(source)?;

        loop {        
            let current_state = *state_stack.back().unwrap();
            let lookahead = scanner.lookahead().map(|token| token.main.tag);
    
            let element = match parse_state(lookahead.as_ref(), current_state, &mut state_stack, &self.language)? {
                TransitionEvent::Shift { current_state, next_state, .. } => {
                    match scanner.shift() {
                        Some(Token { leading, main, trailing }) => {
                            create_green_token(main.tag, main.value, current_state, next_state, &mut cache)?
                        }
                        None => None
                    }
                }
                TransitionEvent::Reduce { syntax_kind: kind, current_state: _current_state, next_state: _next_state, pop_count } => {
                    create_green_node(kind, pop_count, &mut element_stack)?
                }
                TransitionEvent::Accept { current_state: _current_state, syntax_kind: kind } if ! element_stack.is_empty() => {
                    let root = create_green_node(kind, element_stack.len(), &mut element_stack)?
                        .and_then(NodeElement::into_node)
                        .unwrap()
                    ;

                    return Ok(SyntaxTree::new(root, self.language.clone(), intern_cache));
                }
                TransitionEvent::Accept { current_state: _current_state, syntax_kind: _syntax_kind } => {
                    let root = create_error_node()?.into_node().unwrap();

                    return Ok(SyntaxTree::new(root, self.language.clone(), intern_cache));
                }
                TransitionEvent::Error { syntax_kind, failed_state, pop_count, candidate_syntax_kinds } => {
                    todo!()
                }
            };

            element_stack.push_back(element);
        }
    }

    pub fn language(&self) -> &Language {
        &self.language
    }
}

fn parse_state(lookahead: Option<&SyntaxKind>, current_state: usize, state_stack: &mut LinkedList<usize>, language: &Language) -> Result<TransitionEvent, anyhow::Error> {
    let event = match (language.resolve_lookahead_state(lookahead, current_state)?, lookahead) {
        (LookaheadTransition::Shift { next_state }, Some(lookahead)) => {
            let tag = lookahead.clone();

            state_stack.push_back(next_state);
            TransitionEvent::Shift { syntax_kind: tag, next_state: next_state, current_state }   
        }
        (LookaheadTransition::Reduce { pop_count, lhs }, _) => {
            use cstree::Syntax;
            for _ in 0..pop_count {
                state_stack.pop_back();
            }
            
            let peek = *state_stack.back().unwrap();
            let next_state = language.resolve_goto_state(peek, lhs)?;
            let kind = SyntaxKind::from_raw(cstree::RawSyntaxKind(lhs));
            
            state_stack.push_back(next_state);
            TransitionEvent::Reduce { next_state: next_state, current_state, pop_count: pop_count, syntax_kind: kind }
        }
        (LookaheadTransition::Accept { last_kind, .. }, _) => {
            TransitionEvent::Accept { syntax_kind: last_kind, current_state }
        }
        _=> {
            bail!("Unexpected error (current_state: {current_state})");
        }
    };

    Ok(event)
}

fn create_green_token(kind: SyntaxKind, input: Option<String>, current_state: usize, next_state: usize, cache: &mut NodeCache<InternCache>) -> Result<Option<NodeElement>, anyhow::Error> {
    let mut builder = cstree::build::GreenNodeBuilder::<SyntaxKind, InternCache>::with_cache(cache);
    builder.start_node(kind);

    match (kind.is_keyword, kind.is_terminal) {
        (true, true) => {
            builder.static_token(kind);
        }
        (false, true) => {
            let s = input.unwrap_or(kind.text.to_string());
            builder.token(kind, &s);
        }
        _ => {
            bail!("Unexpected shift state (kind: {:?}, input: {:?}, state: {} -> {})", kind, input, current_state, next_state);
        }
    }

    builder.finish_node();
    let node = builder.finish().0.children().next()
        .and_then(|x| x.into_token())
        .map(|x| cstree::util::NodeOrToken::<GreenNode, GreenToken>::Token(x.clone()))
    ;
    
    Ok(node)
}

fn create_green_node(kind: SyntaxKind, pop_count: usize, stack: &mut LinkedList<Option<NodeElement>>) -> Result<Option<NodeElement>, anyhow::Error> {
    use cstree::Syntax;
    let children = stack.split_off(stack.len() - pop_count)
        .into_iter()
        .filter_map(std::convert::identity)
        .map(Into::into)
        .collect::<Vec<_>>()
    ;

    if children.is_empty() {
        return Ok(None);
    }

    let node = cstree::green::GreenNode::new(kind.into_raw(), children);

    Ok(Some(NodeElement::Node(node)))
}

fn create_error_node() -> Result<NodeElement, anyhow::Error> {
    use cstree::Syntax;

    #[cfg(not(feature = "parser_generated"))]
    let kind = SyntaxKind { id: u32::MAX, text: "ILLEGAL", is_keyword: false, is_terminal: false };
    #[cfg(feature = "parser_generated")]
    let kind = syntax_kind::r#ILLEGAL;

    let node = cstree::green::GreenNode::new(kind.into_raw(), vec![]);

    Ok(NodeElement::Node(node))
}

enum AnnotationStatus {
    Resolved,
    Unresolved(Vec<u32>),
}