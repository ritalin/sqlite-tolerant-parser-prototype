use std::collections::{HashMap, LinkedList};
use anyhow::bail;
use cstree::{build::NodeCache, green::{GreenNode, GreenToken}, util::NodeOrToken};
use scanner::{Scanner, Token};
use sqlite_parser_proto::{engine::{alternative_symbols, kinds as syntax_kind}, LookaheadTransition, SyntaxKind, TransitionEvent};
use crate::{InternCache, Language, SyntaxTree, Annotation};


pub struct Parser {
    language: Language,
}

type NodeElement = NodeOrToken::<GreenNode, GreenToken>;

#[derive(Clone, PartialEq, Eq, Hash)]
enum AnnotationKey {
    Node(GreenNode),
    Token(GreenToken),
}

impl From<&NodeElement> for AnnotationKey {
    fn from(value: &NodeElement) -> Self {
        match value {
            NodeOrToken::Node(node) => Self::Node(node.clone()),
            NodeOrToken::Token(node) => Self::Token(node.clone()),
        }
    }
}

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
        let mut node_annotations = HashMap::<AnnotationKey, (Annotation, AnnotationStatus)>::new();

        let mut scanner = Scanner::create(source)?;

        let resolve_rules = HashMap::<(SyntaxKind, SyntaxKind), SyntaxKind>::from_iter([
            ((syntax_kind::r#selcollist, syntax_kind::r#STAR), syntax_kind::r#ASTERISK)
        ]);

        loop {        
            let current_state = *state_stack.back().unwrap();
            let lookahead = scanner.lookahead().map(|token| token.main.tag);
    
            let element = match parse_state(lookahead.as_ref(), current_state, &mut state_stack, &self.language)? {
                TransitionEvent::Shift { current_state, next_state, .. } => {
                    match scanner.shift() {
                        Some(token) => {
                            create_green_token(token, current_state, next_state, &mut cache, &mut node_annotations)?
                        }
                        None => None
                    }
                }
                TransitionEvent::Reduce { syntax_kind: kind, current_state: _current_state, next_state: _next_state, pop_count } => {
                    create_green_node(kind, pop_count, &mut element_stack, &mut node_annotations)?
                }
                TransitionEvent::Accept { current_state: _current_state, syntax_kind: kind } if ! element_stack.is_empty() => {
                    let root = create_green_node(kind, element_stack.len(), &mut element_stack, &mut node_annotations)?
                        .and_then(NodeElement::into_node)
                        .unwrap()
                    ;

                    let root = resolve_anotation_status(&root, &node_annotations, &resolve_rules);

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

fn create_green_token(token: Token, current_state: usize, next_state: usize, cache: &mut NodeCache<InternCache>, annotations: &mut HashMap<AnnotationKey, (Annotation, AnnotationStatus)>) -> Result<Option<NodeElement>, anyhow::Error> {
    let leading = 
        token.leading.map(|items| {
            items.iter().filter_map(|item| create_green_token_internal(item.tag, item.value.as_ref(), current_state, next_state, cache).transpose())
            .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
    ;

    let main = match create_green_token_internal(token.main.tag, token.main.value.as_ref(), current_state, next_state, cache)? {
        Some(main) => Some(vec![main]),
        None => None,
    };

    let trailing = 
        token.trailing.map(|items| {
            items.iter().filter_map(|item| create_green_token_internal(item.tag, item.value.as_ref(), current_state, next_state, cache).transpose())
            .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
    ;

    let children = [leading, main, trailing].into_iter()
        .filter_map(std::convert::identity)
        .flat_map(std::convert::identity)
        .collect::<Vec<_>>()
    ;

    if children.is_empty() {
        return Ok(None)
    }

    use cstree::Syntax;
    let node = GreenNode::new(token.main.tag.into_raw(), children);

    let annotation = Annotation::State;
    let status = AnnotationStatus{ 
        range_from: token.main.offset, 
        len: token.main.len, 
        resolved: alternative_symbols(token.main.tag.id).is_none() 
    };

    annotations.insert(AnnotationKey::Node(node.clone()), (annotation, status));

    Ok(Some(NodeElement::Node(node)))
}

fn create_green_token_internal(kind: SyntaxKind, input: Option<&String>, current_state: usize, next_state: usize, cache: &mut NodeCache<InternCache>) -> Result<Option<NodeElement>, anyhow::Error> {
    let mut builder = cstree::build::GreenNodeBuilder::<SyntaxKind, InternCache>::with_cache(cache);
    builder.start_node(kind);

    match (kind.is_keyword, kind.is_terminal) {
        (true, true) => {
            builder.static_token(kind);
        }
        (false, true) => {
            let s = input.map(String::clone).unwrap_or(kind.text.to_string());
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

fn create_green_node(kind: SyntaxKind, pop_count: usize, stack: &mut LinkedList<Option<NodeElement>>, annotations: &mut HashMap<AnnotationKey, (Annotation, AnnotationStatus)>) -> Result<Option<NodeElement>, anyhow::Error> {
    use cstree::Syntax;
    let children = stack.split_off(stack.len() - pop_count)
        .into_iter()
        .filter_map(std::convert::identity)
        .map(Into::<NodeElement>::into)
        .collect::<Vec<_>>()
    ;

    if children.is_empty() {
        return Ok(None);
    }

    let (offset, len) = children.iter()
        .filter_map(|node| annotations.get(&AnnotationKey::from(node)))
        .fold((0, 0), |(offset, len), (_, status)| {
            (usize::min(status.range_from, offset), status.len + len)
        })
    ;

    let node = cstree::green::GreenNode::new(kind.into_raw(), children);

    let annotation = Annotation::State;
    let staus = AnnotationStatus{ range_from: offset, len: len, resolved: true };

    annotations.insert(AnnotationKey::Node(node.clone()), (annotation, staus));

    Ok(Some(NodeElement::Node(node)))
}

fn resolve_anotation_status(parent_node: &GreenNode, annotations: &HashMap<AnnotationKey, (Annotation, AnnotationStatus)>, resolve_rules: &HashMap<(SyntaxKind, SyntaxKind), SyntaxKind>) -> GreenNode {
    use cstree::Syntax;
    let status_map = annotations.iter()
        .map(|(k, (_, status))| (k.clone(), status))
        .collect::<HashMap<_, _>>()
    ;

    let kind = SyntaxKind::from_raw(parent_node.kind());
    resolve_anotation_status_children(parent_node, &kind, &kind, &status_map, resolve_rules)
}

fn resolve_anotation_status_children(
    parent_node: &GreenNode, lhs: &SyntaxKind, rhs: &SyntaxKind,
    status_map: &HashMap<AnnotationKey, &AnnotationStatus>, 
    resolve_rules: &HashMap<(SyntaxKind, SyntaxKind), SyntaxKind>) -> GreenNode
{
    use cstree::Syntax;
    let mut children = parent_node.children()
        .map(|child| {
            match child {
                NodeOrToken::Node(node) => {
                    let new_child = resolve_anotation_status_children(node, rhs, &SyntaxKind::from_raw(node.kind()), status_map, resolve_rules);
                    NodeElement::Node(new_child)
                }
                NodeOrToken::Token(node) => NodeElement::Token(node.clone()),
            }
        })
        .collect::<Vec<_>>()
    ;

    // rearrange children
    sort_children(&mut children, status_map);

    if let Some(status) = status_map.get(&AnnotationKey::Node(parent_node.clone())) {
        if !status.resolved {
            if let Some(k) = resolve_rules.get(&(*lhs, *rhs)) {
                return GreenNode::new(cstree::RawSyntaxKind(k.id), children);
            }
        }
    }

    GreenNode::new(cstree::RawSyntaxKind(lhs.id), children)
}

fn create_error_node() -> Result<NodeElement, anyhow::Error> {
    use cstree::Syntax;

    let kind = syntax_kind::r#ILLEGAL;
    let node = cstree::green::GreenNode::new(kind.into_raw(), vec![]);

    Ok(NodeElement::Node(node))
}

fn sort_children(children: &mut Vec<NodeElement>, status_map: &HashMap<AnnotationKey, &AnnotationStatus>) {
    children.sort_by(|lhs, rhs| {
        let l_status = status_map.get(&AnnotationKey::from(lhs));
        let r_status = status_map.get(&AnnotationKey::from(rhs));

        match (l_status, r_status) {
            (Some(lhs), Some(rhs)) => lhs.cmp(rhs),
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
}

#[derive(Eq, Ord)]
struct AnnotationStatus {
    range_from: usize,
    len: usize,
    resolved: bool,
}

impl PartialEq for AnnotationStatus {
    fn eq(&self, other: &Self) -> bool {
        self.range_from == other.range_from && self.len == other.len
    }
}

impl PartialOrd for AnnotationStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.range_from.partial_cmp(&other.range_from) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.len.partial_cmp(&other.len)
    }
}