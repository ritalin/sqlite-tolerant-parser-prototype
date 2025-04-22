use std::{collections::HashMap, time::Instant};
use anyhow::bail;
use cstree::{build::NodeCache, green::{GreenNode, GreenToken}, syntax::{SyntaxNode, SyntaxToken}, util::NodeOrToken};
use scanner::{Scanner, Token, TokenItem};
use sqlite_parser_proto::{engine::kinds as syntax_kind, LookaheadTransition, SyntaxKind, TransitionEvent};
use crate::{Annotation, InternCache, Language, NodeElement, NodeType, Recovery, SyntaxTree};


pub struct Parser {
    language: Language,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AnnotationKey {
    pub kind: SyntaxKind, 
    pub offset: usize, 
    pub len: usize,
}

impl From<&SyntaxNode<SyntaxKind>> for AnnotationKey {
    fn from(value: &SyntaxNode<SyntaxKind>) -> Self {
        let range = value.text_range();
        Self {
            kind: value.kind(),
            offset: range.start().into(),
            len: range.len().into(),
        }
    }
}

impl From<&SyntaxToken<SyntaxKind>> for AnnotationKey {
    fn from(value: &SyntaxToken<SyntaxKind>) -> Self {
        let range = value.text_range();
        Self {
            kind: value.kind(),
            offset: range.start().into(),
            len: range.len().into(),
        }
    }
}

enum NodeElementOrError {
    Element{ id: NodeId, element: NodeElement },
    Error{ id: NodeId, element: NodeElement },
}

impl NodeElementOrError {
    fn into_element(id: NodeId, element: NodeElement) -> Self {
        Self::Element { id, element }
    }
    fn into_error(id: NodeId, element: NodeElement) -> Self {
        Self::Error { id, element }
    }
}

thread_local! {
    static ID_GENERATOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
}

pub type NodeId = (std::time::Instant, u64); 

fn next_node_id() -> NodeId {
    let ts = Instant::now();
    let id = ID_GENERATOR.with(|g| g.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    (ts, id)
}

impl Parser {
    pub fn new() -> Self {
        Self {
            language: Default::default(),
        }
    }

    pub fn parse(&self, source: &str) -> Result<SyntaxTree, anyhow::Error> {
        let mut state_stack = vec![0];
        let mut element_stack: Vec<Option<NodeElementOrError>> = vec![];
        let mut intern_cache = InternCache::new();
        let mut cache = NodeCache::with_interner(&mut intern_cache);
        let mut node_annotations = HashMap::<NodeId, (Annotation, AnnotationStatus)>::new();

        let mut scanner = Scanner::create(source)?;

        let resolve_rules = HashMap::<(SyntaxKind, SyntaxKind), SyntaxKind>::from_iter([
            ((syntax_kind::r#selcollist, syntax_kind::r#STAR), syntax_kind::r#ASTERISK)
        ]);

        let root_kind = syntax_kind::r#program;
        let root_member_kind = syntax_kind::r#ecmd;
        let mut root_members = vec![];

        while let Some(lookahead) = scanner.lookahead() {
            if (lookahead.main.tag == syntax_kind::r#EOF) && state_stack.is_empty() {
                let token = create_green_token(lookahead.clone(), lookahead.main.tag, usize::MAX, usize::MAX, &mut cache, &mut node_annotations)?;
                element_stack.push(token.map(|(id, element)| NodeElementOrError::into_element(id, element)));
                let state = state_stack.pop().unwrap_or(usize::MAX);
                let root_member = create_green_node(root_member_kind, state, state, element_stack.len(), &mut element_stack, &mut node_annotations)?;
                root_members.push(root_member);
                break;
            }

            match parse_internal(&mut scanner, &mut state_stack, &mut element_stack, &mut node_annotations, &mut cache, &self.language)? {
                NodeGenerated::Node(element) => {
                    element_stack.push(element);
                }
                NodeGenerated::Root(root) => {
                    let tree = create_syntax_tree(root, intern_cache, node_annotations, &resolve_rules, &self.language);
                    return Ok(tree);
                }
                NodeGenerated::RootMember(element) => {
                    root_members.push(element);

                    state_stack.clear();
                    element_stack.clear();
                }
                // NodeGenerated::Error { error, recovered } => {
                //     error_nodes.push(error);

                //     match *recovered {
                //         Some(NodeGenerated::Node(element)) => {
                //             element_stack.push(element);
                //         }
                //         Some(NodeGenerated::Root(root)) => {
                //             let tree = create_syntax_tree(root, intern_cache, node_annotations, &resolve_rules, &self.language);
                //             return Ok(tree);        
                //         }
                //         Some(NodeGenerated::RootMember(element)) => {
                //             root_members.push(element);

                //             state_stack.clear();
                //             element_stack.clear();
                //         }
                //         _ => {}
                //     }
                // }
            }
        }

        let root = GreenNode::new(
            cstree::RawSyntaxKind(root_kind.id), 
            root_members.into_iter().filter_map(std::convert::identity).map(|(_, member)| member).collect::<Vec<_>>()
        );
        let tree = create_syntax_tree(root, intern_cache, node_annotations, &resolve_rules, &self.language);
        Ok(tree)
    }

    pub fn language(&self) -> &Language {
        &self.language
    }
}

enum NodeGenerated {
    Node(Option<NodeElementOrError>),
    Root(GreenNode),
    RootMember(Option<(NodeId, NodeElement)>),
}

fn parse_internal(
    scanner: &mut Scanner, 
    state_stack: &mut Vec<usize>, 
    element_stack: &mut Vec<Option<NodeElementOrError>>,
    node_annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>, 
    cache: &mut NodeCache<InternCache>,
    language: &Language) -> Result<NodeGenerated, anyhow::Error> 
{
    let root_member_kind = syntax_kind::r#ecmd;
    let terminte_kind = syntax_kind::r#SEMI;
    
    let current_state = *state_stack.last().unwrap();
    let lookahead = scanner.lookahead().cloned();
    let main_kind = lookahead.as_ref().map(|token| token.main.tag);

    match parse_state(main_kind.as_ref(), current_state, state_stack, language)? {
        TransitionEvent::Shift { current_state, next_state, .. } => {
            match scanner.shift() {
                Some(token) if token.main.tag == terminte_kind => {
                    let kind = token.main.tag;
                    let token = create_green_token(token, kind, current_state, next_state, cache, node_annotations)?;
                    element_stack.push(token.map(|(id, element)| NodeElementOrError::into_element(id, element)));
                    let root_member = create_green_node(root_member_kind, current_state, next_state, element_stack.len(), element_stack, node_annotations)?;
                    Ok(NodeGenerated::RootMember(root_member))
                }
                Some(token) => {
                    let kind = token.main.tag;
                    let node = create_green_token(token, kind, current_state, next_state, cache, node_annotations)?;
                    Ok(NodeGenerated::Node(node.map(|(id, element)| NodeElementOrError::into_element(id, element))))
                }
                None => Ok(NodeGenerated::Node(None))
            }
        }
        TransitionEvent::Reduce { syntax_kind: kind, current_state, next_state, pop_count } => {
            let node = create_green_node(kind, current_state, next_state, pop_count, element_stack, node_annotations)?;
            Ok(NodeGenerated::Node(node.map(|(id, element)| NodeElementOrError::into_element(id, element))))
        }
        TransitionEvent::Accept { current_state, syntax_kind: kind } if ! element_stack.is_empty() => {
            let root = create_green_node(kind, current_state, current_state, element_stack.len(), element_stack, node_annotations)?
                .and_then(|(_, element)| NodeElement::into_node(element))
                .unwrap()
            ;

            Ok(NodeGenerated::Root(root))
        }
        TransitionEvent::Accept { current_state: _current_state, syntax_kind: _syntax_kind } => {
            let root = create_fatal_error_node()?.into_node().unwrap();

            Ok(NodeGenerated::Root(root))
        }
        TransitionEvent::Error { failed_state, .. } => {
            let mut journals = vec![];
            if let Some(journal) = try_state_recovery_by_drop(scanner, state_stack, language) {
                journals.push(journal);
            }

            let recovered = match journals.into_iter().max_by(|lhs, rhs| lhs.events.len().cmp(&rhs.events.len())) {
                Some(Journal{ recovery, events }) if recovery == Recovery::Delete => {
                    let next_state = events.iter().filter_map(|event| event.next_state()).next().unwrap_or_default();
                    let error = create_drop_error_node(scanner.shift(), failed_state, next_state, cache, node_annotations)?;
                    element_stack.push(error.map(|(id, element)| NodeElementOrError::into_error(id, element)));

                    replay_translation_event(&events, scanner, state_stack, element_stack, node_annotations, cache)?
                }
                Some(Journal{ events, .. }) => {
                    let token_offset = lookahead.map(|token| token.offset_start()).unwrap_or_default();
                    let next_state = events.iter().filter_map(|event| event.next_state()).next().unwrap_or_default();
                    let error = create_blank_error_node(token_offset, failed_state, next_state, node_annotations)?;
                    element_stack.push(error.map(|(id, element)| NodeElementOrError::into_error(id, element)));

                    replay_translation_event(&events, scanner, state_stack, element_stack, node_annotations, cache)?
                }
                None => {
                    // FIXME: FatalError
                    todo!()
                }
            };

            match recovered {
                Some(recovered) => {
                    Ok(recovered)
                }
                None => {
                    bail!("Error recover failed");
                }
            }
        }
    }
}

fn parse_state(lookahead: Option<&SyntaxKind>, current_state: usize, state_stack: &mut Vec<usize>, language: &Language) -> Result<TransitionEvent, anyhow::Error> {
    let event = match (language.resolve_lookahead_state(lookahead, current_state), lookahead) {
        (Ok(LookaheadTransition::Shift { next_state }), Some(lookahead)) => {
            let tag = lookahead.clone();

            state_stack.push(next_state);
            TransitionEvent::Shift { syntax_kind: tag, next_state, current_state }  
        }
        (Ok(LookaheadTransition::Reduce { pop_count, lhs }), _) => {
            use cstree::Syntax;
            for _ in 0..pop_count {
                state_stack.pop();
            }
            
            let peek = *state_stack.last().unwrap();
            let next_state = language.resolve_goto_state(peek, lhs)?;
            let kind = SyntaxKind::from_raw(cstree::RawSyntaxKind(lhs));
            
            state_stack.push(next_state);
            TransitionEvent::Reduce { next_state, current_state, pop_count, syntax_kind: kind }
        }
        (Ok(LookaheadTransition::Accept { last_kind, .. }), _) => {
            TransitionEvent::Accept { syntax_kind: last_kind, current_state }
        }
        _=> {
            TransitionEvent::Error { syntax_kind: lookahead.cloned(), failed_state: current_state }
        }
    };

    Ok(event)
}

fn create_green_token(token: Token, main_kind: SyntaxKind, current_state: usize, next_state: usize, cache: &mut NodeCache<InternCache>, annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<Option<(NodeId, NodeElement)>, anyhow::Error> {
    let annotation = Annotation { node_type: crate::NodeType::TokenSet, recovery: None };
    let status = AnnotationStatus{ 
        kind: token.main.tag,
        range_from: token.offset_start(), 
        len: token.token_len(), 
    };
    
    let leading = 
        token.leading.map(|items| {
            items.iter().filter_map(|item| create_green_token_internal(item, NodeType::LeadingToken, current_state, next_state, annotations, cache).transpose())
            .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
    ;

    let main = match create_green_token_internal(&token.main, NodeType::MainToken, current_state, next_state, annotations, cache)? {
        Some(main) => Some(vec![main]),
        None => None,
    };

    let trailing = 
        token.trailing.map(|items| {
            items.iter().filter_map(|item| create_green_token_internal(item, NodeType::TrailingToken, current_state, next_state, annotations, cache).transpose())
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
    let node = GreenNode::new(main_kind.into_raw(), children);
    let id = next_node_id();

    annotations.insert(id, (annotation, status));

    Ok(Some((id, NodeElement::Node(node))))
}

fn create_green_token_internal(token: &TokenItem, node_type: NodeType, current_state: usize, next_state: usize, annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>, cache: &mut NodeCache<InternCache>) -> Result<Option<NodeElement>, anyhow::Error> {
    let mut builder = cstree::build::GreenNodeBuilder::<SyntaxKind, InternCache>::with_cache(cache);
    builder.start_node(token.tag);

    match (token.tag.is_keyword, token.tag.is_terminal) {
        (true, true) => {
            builder.static_token(token.tag);
        }
        (false, true) => {
            let s = token.value.clone().unwrap_or(token.tag.text.to_string());
            builder.token(token.tag, &s);
        }
        _ => {
            bail!("Unexpected shift state (kind: {:?}, input: {:?}, state: {} -> {})", token.tag, token.value, current_state, next_state);
        }
    }

    builder.finish_node();
    let node = builder.finish().0.children().next()
        .and_then(|x| x.into_token())
        .map(|x| cstree::util::NodeOrToken::<GreenNode, GreenToken>::Token(x.clone()))
    ;

    match node.as_ref() {
        Some(NodeElement::Token(_)) => {
            let annotation = Annotation { node_type, recovery: None };
            let status = AnnotationStatus{ 
                kind: token.tag,
                range_from: token.offset, 
                len: token.len, 
            };
            annotations.insert(next_node_id(), (annotation, status));        
        }
        _ => {}
    }

    Ok(node)
}

fn create_green_node(kind: SyntaxKind, current_state: usize, next_state: usize, pop_count: usize, stack: &mut Vec<Option<NodeElementOrError>>, annotation_map: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<Option<(NodeId, NodeElement)>, anyhow::Error> {
    use cstree::Syntax;
    let (children, id_set) = pop_elements(stack, pop_count);

    if children.is_empty() {
        return Ok(None);
    }

    let (offset, len) = id_set.into_iter()
        .filter_map(|child_id| annotation_map.get(&child_id))
        .fold((usize::MAX, 0), |(offset, len), (_, status)| {
            (usize::min(status.range_from, offset), status.len + len)
        })
    ;

    let node = cstree::green::GreenNode::new(kind.into_raw(), children);
    let id = next_node_id();

    let annotation = Annotation { node_type: crate::NodeType::Node, recovery: None };
    let staus = AnnotationStatus{ kind, range_from: offset, len };

    annotation_map.insert(id, (annotation, staus));

    Ok(Some((id, NodeElement::Node(node))))
}

fn pop_elements(element_stack: &mut Vec<Option<NodeElementOrError>>, mut pop_count: usize) -> (Vec<NodeElement>, Vec<NodeId>) {
    let mut elements = Vec::with_capacity(pop_count + 1);

    while pop_count > 0 {
        match element_stack.pop() {
            Some(Some(NodeElementOrError::Element{ id, element })) => {
                elements.push((element, id));
                pop_count -= 1;
            }
            Some(None) => {
                pop_count -= 1;
            }
            Some(Some(NodeElementOrError::Error{ id, element })) => {
                elements.push((element, id));
                pop_count -= 1;
            }
            _ => {}
        }
        if pop_count == 0 { break }
    }

    if let Some(Some(NodeElementOrError::Error{ id, element })) = element_stack.last() {
        elements.push((element.clone(), id.clone()));
        element_stack.pop();
    }

    elements.reverse();
    elements.into_iter().unzip()
}

fn resolve_anotation_status(
    parent_node: SyntaxNode<SyntaxKind>, 
    annotations: &mut HashMap<AnnotationKey, (NodeId, Annotation)>, 
    resolve_rules: &HashMap<(SyntaxKind, SyntaxKind), SyntaxKind>) -> GreenNode 
{
    let kind = parent_node.kind();
    resolve_anotation_status_children(parent_node, kind, kind, annotations, resolve_rules)
}

fn resolve_anotation_status_children(
    parent_node: SyntaxNode<SyntaxKind>, parent_kind: SyntaxKind, kind: SyntaxKind,
    annotations: &mut HashMap<AnnotationKey, (NodeId, Annotation)>, 
    resolve_rules: &HashMap<(SyntaxKind, SyntaxKind), SyntaxKind>) -> GreenNode
{
    let children = parent_node.children_with_tokens()
        .map(|child| match child {
            NodeOrToken::Node(node) => {
                let new_node = resolve_anotation_status_children(node.clone(), kind, child.kind(), annotations, resolve_rules);
                NodeElement::Node(new_node)
            }
            NodeOrToken::Token(node) => {
                NodeElement::Token(node.green().clone())
            }
        })
        .collect::<Vec<_>>()
    ;

    match resolve_rules.get(&(parent_kind, kind)) {
        Some(new_kind) => {
            let new_node = GreenNode::new(cstree::RawSyntaxKind(new_kind.id), children);
            let key = AnnotationKey::from(&parent_node);

            if let Some(annotation) = annotations.get(&key).cloned() {
                let new_key = AnnotationKey{ kind: new_kind.clone(), offset: key.offset, len: key.len };
                annotations.entry(new_key).or_insert_with(|| {
                    annotation
                });
            }
            new_node
        }
        None => {
            GreenNode::new(cstree::RawSyntaxKind(kind.id), children)
        }
    }
}

fn create_drop_error_node(lookahead: Option<Token>, state: usize, next_state: usize, cache: &mut NodeCache<InternCache>, annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<Option<(NodeId, NodeElement)>, anyhow::Error> {
    let Some(lookahead) = lookahead else {
        return create_blank_error_node(0, state, next_state, annotations);
    };
    let kind = lookahead.main.tag;

    create_green_token(lookahead, kind, state, next_state, cache, annotations)
}

fn create_blank_error_node(offset: usize, state: usize, next_state: usize, annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<Option<(NodeId, NodeElement)>, anyhow::Error> {
    use cstree::Syntax;

    // FIXME: need blank token kind
    let kind = syntax_kind::r#ILLEGAL;
    let node = cstree::green::GreenNode::new(kind.into_raw(), vec![]);
    let id = next_node_id();

    let annotation = Annotation { node_type: crate::NodeType::Node, recovery: None };
    let staus = AnnotationStatus{ kind, range_from: offset, len: 0 };

    annotations.insert(id, (annotation, staus));
    Ok(Some((id, NodeElement::Node(node))))
}

fn create_fatal_error_node() -> Result<NodeElement, anyhow::Error> {
    use cstree::Syntax;

    let kind = syntax_kind::r#ILLEGAL;
    let node = cstree::green::GreenNode::new(kind.into_raw(), vec![]);

    Ok(NodeElement::Node(node))
}

fn replay_translation_event(
    events: &[TransitionEvent], scanner: &mut Scanner, 
    state_stack: &mut Vec<usize>, 
    element_stack: &mut Vec<Option<NodeElementOrError>>,
    node_annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>, 
    cache: &mut NodeCache<InternCache>) -> Result<Option<NodeGenerated>, anyhow::Error> 
{
    for event in events {
        match event {
            TransitionEvent::Shift { syntax_kind, current_state, next_state } => {
                let token = scanner.shift().unwrap();
                let node = create_green_token(token, *syntax_kind, *current_state, *next_state, cache, node_annotations)?;

                element_stack.push(node.map(|(id, element)| NodeElementOrError::into_element(id, element)));
                state_stack.push(*next_state);
            }
            TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count } => {
                let node = create_green_node(*syntax_kind, *current_state, *next_state, *pop_count, element_stack, node_annotations)?;
                
                element_stack.push(node.map(|(id, element)| NodeElementOrError::into_element(id, element)));
                state_stack.push(*next_state);
            }
            TransitionEvent::Accept { syntax_kind, current_state } => {
                let root = create_green_node(*syntax_kind, *current_state, *current_state, element_stack.len(), element_stack, node_annotations)?
                    .and_then(|(_, element)| NodeElement::into_node(element))
                    .unwrap()
                ;

                return Ok(Some(NodeGenerated::Root(root)));
            }
            TransitionEvent::Error { .. } => {
                return Ok(None)
            }
        }
    }

    match element_stack.pop() {
        Some(node) => {
            Ok(Some(NodeGenerated::Node(node)))
        }
        None => Ok(None)
    }
}

fn create_syntax_tree(root: GreenNode, intern_cache: InternCache, node_annotations: HashMap<NodeId, (Annotation, AnnotationStatus)>, resolve_rules: &HashMap<(SyntaxKind, SyntaxKind), SyntaxKind>, language: &Language) -> SyntaxTree {
    let mut annotations = node_annotations.into_iter()
        .map(|(id, (annotation, status))| {
            let key = AnnotationKey{ kind: status.kind, offset: status.range_from, len: status.len };
            (key, (id, annotation))
        })
        .collect::<HashMap<_, _>>()
    ;
    let root = resolve_anotation_status(SyntaxNode::new_root(root), &mut annotations, &resolve_rules);
    
    let red_root = SyntaxNode::new_root_with_resolver(root, intern_cache.clone());
    
    let id = next_node_id();
    let key = AnnotationKey::from(red_root.syntax());
    let annotation = Annotation { node_type: crate::NodeType::Node, recovery: None };
    annotations.insert(key, (id, annotation));

    SyntaxTree::new(red_root, language.clone(), intern_cache.clone(), annotations)
}

#[derive(Eq, Clone, Debug)]
struct AnnotationStatus {
    kind: SyntaxKind,
    range_from: usize,
    len: usize,
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
impl Ord for AnnotationStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.range_from.cmp(&other.range_from) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.len.cmp(&other.len)
    }
}

struct Journal {
    recovery: Recovery,
    events: Vec<TransitionEvent>,
}

fn try_state_recovery_by_drop(scanner: &mut Scanner, state_stack: &Vec<usize>, language: &Language) -> Option<Journal> {
    let scope = scanner.scope();
    let mut events = Vec::with_capacity(64);
    let mut state_stack = state_stack.clone();
    
    while let Some(lookahead) = scanner.lookahead() {
        if (lookahead.main.tag == syntax_kind::r#SEMI) || (lookahead.main.tag == syntax_kind::r#EOF) {
            break
        }
        let current_state = *state_stack.last().unwrap();
    
        match parse_state(Some(&lookahead.main.tag), current_state, &mut state_stack, language) {
            Ok(TransitionEvent::Error { .. }) => return None,
            Err(_) => return None,
            Ok(TransitionEvent::Shift { syntax_kind, current_state, next_state }) => {
                scanner.shift();
                events.push(TransitionEvent::Shift { syntax_kind, current_state, next_state });
            }
            Ok(TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count }) if pop_count == 0 => {
                events.push(TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count });
            }
            Ok(TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count }) => {
                events.push(TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count });
                break;
            }
            Ok(TransitionEvent::Accept { current_state, syntax_kind }) => {
                events.push(TransitionEvent::Accept { current_state, syntax_kind });
                break;
            }
        }
    }
    scanner.revert(scope);
    
    Some(Journal { recovery: Recovery::Delete, events })
}

fn try_state_recovery_by_shift(scanner: &mut Scanner, start_state: usize, language: &Language) -> Option<Journal> {

    let scope = scanner.scope();
    scanner.revert(scope);
    todo!()
}
