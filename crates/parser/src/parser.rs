use std::{collections::HashMap, rc::Rc, time::Instant};
use anyhow::bail;
use cactus::Cactus;
use cstree::{build::NodeCache, green::{GreenNode, GreenToken}, syntax::{ResolvedNode, SyntaxNode, SyntaxToken}, text::TextSize, util::NodeOrToken, Syntax};
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
    pub is_node: bool,
}

impl From<&SyntaxNode<SyntaxKind>> for AnnotationKey {
    fn from(value: &SyntaxNode<SyntaxKind>) -> Self {
        let range = value.text_range();
        Self {
            kind: value.kind(),
            offset: range.start().into(),
            len: range.len().into(),
            is_node: true,
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
            is_node: false,
        }
    }
}

#[derive(Clone)]
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

    fn kind(&self) -> SyntaxKind {
        let kind_raw = match self {
            NodeElementOrError::Element { element, .. } => element.kind(),
            NodeElementOrError::Error { element, .. } => element.kind(),
        };

        SyntaxKind::from_raw(kind_raw)
    }
}

#[derive(Clone)]
struct StateStack {
    initial: usize,
    stack: Cactus<usize>,
    checkpoint: Cactus<usize>,
}

impl StateStack {
    pub fn new(initial: usize) -> Self {
        Self {
            initial,
            stack: Cactus::new().child(initial),
            checkpoint: Cactus::new()
        }
    }

    #[inline]
    pub fn push(&mut self, val: usize) {
        self.stack = self.stack.child(val);
    } 

    pub fn pop(&mut self) -> Option<usize> {
        let val = self.stack.val().cloned();
        self.stack = self.stack.parent().unwrap_or_default();
        val
    }

    pub fn pop_n(&mut self, mut count: usize) {
        while count > 0 {
            let Some(parent) = self.stack.parent() else { break };
            self.stack = parent;
            count -= 1;
        }

        assert!(count == 0);
    }

    #[inline]
    pub fn peek(&self) -> Option<&usize> {
        self.stack.val()
    }

    pub fn current_state(&self) -> usize {
        self.stack.parent().and_then(|v| v.val().cloned()).unwrap_or(self.initial)
    }

    pub fn reset(&mut self) {
        self.stack = Cactus::new().child(self.initial);
        self.checkpoint = Cactus::new();
    }

    pub fn mark_checkpoint(&mut self, val: usize) -> usize {
        self.checkpoint = self.checkpoint.child(val);
        val
    }

    pub fn resolve_checkpoint(&mut self, mut pop_count: usize) -> usize {
        while pop_count > 1 {
            self.checkpoint = self.checkpoint.parent().unwrap_or_default();
            pop_count -= 1;
        }
        self.checkpoint.val().cloned().unwrap_or_else(|| self.current_state())
    }

    // #[inline]
    // pub fn is_empty(&self) -> bool {
    //     self.stack.is_empty()
    // }

    pub fn values(&self) -> Vec<usize> {
        let mut values = vec![];

        let mut next_node = self.stack.clone();
        while let Some(v) = next_node.val() {
            values.push(*v);
            
            let Some(node) = next_node.parent() else {
                break
            };
            next_node = node;
        }

        values
    }
}

struct RecoveryPenalty {
    delete_slot: usize,
    shift_limit: usize,
    shift_decay: usize,
    next_shift_decay: usize,
}

impl RecoveryPenalty {
    pub fn accept_delete(&mut self, used_slot: usize) {
        self.delete_slot -= used_slot;
    }
    pub fn accept_shift(&mut self) {
        self.shift_decay = self.next_shift_decay;
        self.next_shift_decay <<= 1;
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

    pub fn parse(&self, source: String) -> Result<SyntaxTree, anyhow::Error> {
        let mut state_stack = StateStack::new(0);
        let mut element_stack: Vec<Option<NodeElementOrError>> = vec![];
        let mut intern_cache = InternCache::new();
        let mut cache = NodeCache::with_interner(&mut intern_cache);
        let mut node_annotations = HashMap::<NodeId, (Annotation, AnnotationStatus)>::new();

        let mut scanner = Scanner::create(source, 0);

        let resolve_rules = HashMap::<(SyntaxKind, SyntaxKind), SyntaxKind>::from_iter([
            ((syntax_kind::r#selcollist, syntax_kind::r#STAR), syntax_kind::r#ASTERISK)
        ]);

        let root_kind = syntax_kind::r#program;
        let root_member_kind = syntax_kind::r#ecmd;
        let mut root_members = vec![];
        let mut penalty = RecoveryPenalty { 
            delete_slot: 3, 
            shift_limit: 9,
            shift_decay: 0, 
            next_shift_decay: 2, 
        };

        while let Some(lookahead) = scanner.lookahead() {
            if lookahead.main.tag == syntax_kind::r#EOF {
                let token = create_green_token(lookahead.clone(), lookahead.main.tag, 0, &mut cache, &mut node_annotations)?;
                element_stack.push(token.map(|(id, element)| NodeElementOrError::into_element(id, element)));
                let state = state_stack.pop().unwrap_or(0);
                let root_member = create_green_node(root_member_kind, state, element_stack.len(), &mut element_stack, &mut node_annotations)?;
                root_members.push(root_member);
                break;
            }

            match parse_internal(&mut scanner, &mut state_stack, &mut element_stack, &mut node_annotations, &mut cache, &mut penalty, &self.language)? {
                NodeGenerated::Node(Some((_, element))) => {
                    element_stack.push(Some(element));
                }
                NodeGenerated::Node(None) => {
                    element_stack.push(None);
                }
                NodeGenerated::Root(_, id, root) => {
                    let tree = create_syntax_tree(root, id, intern_cache, node_annotations, &resolve_rules, &self.language);
                    return Ok(tree);
                }
                NodeGenerated::RootMember(Some((_, id, element))) => {
                    root_members.push(Some((id, element)));

                    state_stack.reset();
                    element_stack.clear();
                }
                NodeGenerated::Fatal(id, element) => {
                    root_members.push(Some((id, element)));
                }
                _ => {}
            }
        }

        let root = GreenNode::new(
            cstree::RawSyntaxKind(root_kind.id), 
            root_members.into_iter().filter_map(std::convert::identity).map(|(_, member)| member).collect::<Vec<_>>()
        );
        let tree = create_syntax_tree(root, next_node_id(), intern_cache, node_annotations, &resolve_rules, &self.language);
        Ok(tree)
    }

    pub fn language(&self) -> &Language {
        &self.language
    }

    pub fn incremental(&self, tree: &SyntaxTree, edit: EditScope) -> Result<IncrementalParser, anyhow::Error> {
        IncrementalParser::create(tree, edit)
    }
}

enum NodeGenerated {
    Node(Option<(SyntaxKind, NodeElementOrError)>),
    Root(SyntaxKind, NodeId, GreenNode),
    RootMember(Option<(SyntaxKind, NodeId, NodeElement)>),
    Fatal(NodeId, NodeElement),
}

fn parse_internal(
    scanner: &mut Scanner, 
    state_stack: &mut StateStack, 
    element_stack: &mut Vec<Option<NodeElementOrError>>,
    node_annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>, 
    cache: &mut NodeCache<InternCache>,
    penalty: &mut RecoveryPenalty,
    language: &Language) -> Result<NodeGenerated, anyhow::Error> 
{
    let root_member_kind = syntax_kind::r#ecmd;
    let terminte_kind = syntax_kind::r#SEMI;
    
    let current_state = state_stack.peek().unwrap();
    let lookahead = scanner.lookahead().cloned();
    let main_kind = lookahead.as_ref().map(|token| token.main.tag);

    match parse_state(main_kind.as_ref(), *current_state, state_stack, language, true)? {
        TransitionEvent::Shift { current_state, .. } => {
            match scanner.shift() {
                Some(token) if token.main.tag == terminte_kind => {
                    let kind = token.main.tag;
                    let token = create_green_token(token, kind, current_state, cache, node_annotations)?;
                    element_stack.push(token.map(|(id, element)| NodeElementOrError::into_element(id, element)));
                    let root_member = create_green_node(root_member_kind, current_state, element_stack.len(), element_stack, node_annotations)?;
                    Ok(NodeGenerated::RootMember(root_member.map(|(id, node)| (kind, id, node))))
                }
                Some(token) => {
                    let kind = token.main.tag;
                    let node = create_green_token(token, kind, current_state, cache, node_annotations)?;
                    Ok(NodeGenerated::Node(node.map(|(id, element)| (kind, NodeElementOrError::into_element(id, element)))))
                }
                None => Ok(NodeGenerated::Node(None))
            }
        }
        TransitionEvent::Reduce { syntax_kind: kind, current_state, pop_count, .. } => {
            // eprintln!("[DEBUG] kind: {}, nodes: [{:?}]", kind.text, syntax_kind_from_node(&element_stack));
            let current_state = if pop_count > 0 { state_stack.resolve_checkpoint(pop_count) } else { state_stack.mark_checkpoint(current_state) };
            let node = create_green_node(kind, current_state, pop_count, element_stack, node_annotations)?;
            Ok(NodeGenerated::Node(node.map(|(id, element)| (kind, NodeElementOrError::into_element(id, element)))))
        }
        TransitionEvent::Accept { current_state: _current_state, syntax_kind: kind } if element_stack.is_empty() => {
            // For empty source
            let lookahead = Token { leading: None, main: TokenItem { tag: syntax_kind::r#EOF, offset: 0, len: 0, value: None }, trailing: None };
            let root_member = create_green_token(lookahead, syntax_kind::r#EOF, 0, cache, node_annotations)?
                .map(|(id, node)| (kind, id, node))
            ;

            Ok(NodeGenerated::RootMember(root_member))
        }
        TransitionEvent::Accept { current_state, syntax_kind: kind } => {
            let root = create_green_node(kind, current_state, element_stack.len(), element_stack, node_annotations)?
                .and_then(|(_, element)| NodeElement::into_node(element))
                .unwrap()
            ;

            Ok(NodeGenerated::Root(kind, next_node_id(), root))
        }
        TransitionEvent::Error { failed_state, .. } => {
            eprintln!("(start recovery) --------------------------------------------------------------------------------");
            let delete_candidate = try_state_recovery_by_drop(scanner, state_stack, failed_state, penalty, language)?;
            let shift_candidate = try_state_recovery_by_shift(scanner, state_stack, failed_state, penalty, language)?;

            let recovered = match (delete_candidate, shift_candidate) {
                (Some((error_journal, delete_journal)), Some((_, shift_journal))) if delete_journal.score() > shift_journal.score() => {
                    penalty.accept_delete(error_journal.events.len());
                    // Won by delete
                    eprintln!("Won by delete#1");
                    replay_delete_recovery(&error_journal.events, &delete_journal.events, scanner, state_stack, element_stack, node_annotations, cache)?
                }
                (Some(_), Some((error_journal, shift_journal))) => {
                    penalty.accept_shift();
                    // Won by shift
                    eprintln!("Won by shift#1");
                    replay_shift_recovery(&error_journal.events, &shift_journal.events, scanner, state_stack, element_stack, node_annotations, cache)?
                }
                (Some((error_journal, delete_journal)), None) => {
                    penalty.accept_delete(error_journal.events.len());
                    // Won by delete
                    eprintln!("Won by delete#2");
                    replay_delete_recovery(&error_journal.events, &delete_journal.events, scanner, state_stack, element_stack, node_annotations, cache)?
                }
                (None, Some((error_journal, shift_journal))) => {
                    penalty.accept_shift();
                    // Won by shift
                    eprintln!("Won by shift#2");
                    replay_shift_recovery(&error_journal.events, &shift_journal.events, scanner, state_stack, element_stack, node_annotations, cache)?
                }
                (None, None) => {
                    // Fatal Error
                    eprintln!("calcel recovery");
                    let kind = syntax_kind::r#ILLEGAL;
                    let (id, node) = create_fatal_error_node(scanner, kind, failed_state, element_stack, cache, node_annotations)?;
                    Some(NodeGenerated::Fatal(id, node))
                }
            };

            eprintln!("(finish recovery) --------------------------------------------------------------------------------");

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

#[allow(unused)]
fn syntax_kind_from_node(elements: &[Option<NodeElementOrError>]) -> Vec<String> {
    elements.iter()
    .map(|x| match x {
        Some(NodeElementOrError::Element { element, .. }) => {
            SyntaxKind::from_raw(element.kind()).text.to_string()
        }
        Some(NodeElementOrError::Error { element, .. }) => {
            SyntaxKind::from_raw(element.kind()).text.to_string()
        }
        None => "(opt)".to_string(),
    })
    .collect()
}

fn parse_state(lookahead: Option<&SyntaxKind>, current_state: usize, state_stack: &mut StateStack, language: &Language, log_enabled: bool) -> Result<TransitionEvent, anyhow::Error> {
    let event = match (language.resolve_lookahead_state(lookahead, current_state), lookahead) {
        (Ok(LookaheadTransition::Shift { next_state }), Some(lookahead)) => {
            let tag = lookahead.clone();

            state_stack.push(next_state);
            let current_state = state_stack.mark_checkpoint(current_state);
            if log_enabled { eprintln!("[DEBUG] Shift/kind: {}, push: {:?}", tag.text, state_stack.values()); }
            TransitionEvent::Shift { syntax_kind: tag, next_state, current_state }  
        }
        (Ok(LookaheadTransition::Reduce { pop_count, lhs }), _) => {
            state_stack.pop_n(pop_count);

            use cstree::Syntax;
            let peek = *state_stack.peek().unwrap();
            let next_state = language.resolve_goto_state(peek, lhs)?;
            let kind = SyntaxKind::from_raw(cstree::RawSyntaxKind(lhs));
            
            state_stack.push(next_state);
            if log_enabled { eprintln!("[DEBUG] Reduce/kind: {}, pop({})&push: {:?}", kind.text, pop_count, state_stack.values()); }
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

fn create_green_token(token: Token, main_kind: SyntaxKind, current_state: usize, cache: &mut NodeCache<InternCache>, annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<Option<(NodeId, NodeElement)>, anyhow::Error> {
    match create_green_token_items(&token, main_kind, current_state, cache, annotations)? {
        Some(node) => {
            let annotation = Annotation { node_type: NodeType::TokenSet, state: current_state, recovery: None };
            let status = AnnotationStatus::new(&token);
            let id = next_node_id();
        
            annotations.insert(id, (annotation, status));
            Ok(Some((id, node)))
        }
        None => Ok(None),
    }
}

fn create_green_token_items(token: &Token, main_kind: SyntaxKind, current_state: usize, cache: &mut NodeCache<InternCache>, annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<Option< NodeElement>, anyhow::Error> {
    let leading = 
        token.leading.as_ref().map(|items| {
            items.iter().filter_map(|item| create_green_token_internal(item, NodeType::LeadingToken, current_state, annotations, cache).transpose())
            .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
    ;

    let main = match create_green_token_internal(&token.main, NodeType::MainToken, current_state, annotations, cache)? {
        Some(main) => Some(vec![main]),
        None => None,
    };

    let trailing = 
        token.trailing.as_ref().map(|items| {
            items.iter().filter_map(|item| create_green_token_internal(item, NodeType::TrailingToken, current_state, annotations, cache).transpose())
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

    Ok(Some(NodeElement::Node(node)))
}

fn create_green_token_internal(token: &TokenItem, node_type: NodeType, current_state: usize, annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>, cache: &mut NodeCache<InternCache>) -> Result<Option<NodeElement>, anyhow::Error> {
    let mut builder = cstree::build::GreenNodeBuilder::<SyntaxKind, InternCache>::with_cache(cache);
    builder.start_node(token.tag);

    match (token.tag.is_keyword, token.tag.is_terminal) {
        (true, true) => {
            builder.static_token(token.tag);
        }
        (false, true) => {
            let s = token.value.clone().unwrap_or("".to_string());
            builder.token(token.tag, &s);
        }
        _ => {
            bail!("Unexpected shift state (kind: {:?}, input: {:?}, state: {})", token.tag, token.value, current_state);
        }
    }

    builder.finish_node();
    let node = builder.finish().0.children().next()
        .and_then(|x| x.into_token())
        .map(|x| cstree::util::NodeOrToken::<GreenNode, GreenToken>::Token(x.clone()))
    ;

    match node.as_ref() {
        Some(NodeElement::Token(_)) => {
            let annotation = Annotation { node_type, state: current_state, recovery: None };
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

fn create_green_node(kind: SyntaxKind, current_state: usize, pop_count: usize, stack: &mut Vec<Option<NodeElementOrError>>, annotation_map: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<Option<(NodeId, NodeElement)>, anyhow::Error> {
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

    let annotation = Annotation { node_type: crate::NodeType::Node, state: current_state, recovery: None };
    let staus = AnnotationStatus{ kind, range_from: offset, len };

    annotation_map.insert(id, (annotation, staus));

    Ok(Some((id, NodeElement::Node(node))))
}

fn pop_elements(element_stack: &mut Vec<Option<NodeElementOrError>>, mut pop_count: usize) -> (Vec<NodeElement>, Vec<NodeId>) {
    assert!(pop_count <= element_stack.len(), "pop_count: {}, stack/len: {}", pop_count, element_stack.len());
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
                let new_key = AnnotationKey{ kind: new_kind.clone(), ..key };
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

fn create_drop_error_node(lookahead: Option<Token>, state: usize, cache: &mut NodeCache<InternCache>, annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<Option<(NodeId, NodeElement)>, anyhow::Error> {
    let Some(lookahead) = lookahead else {
        return Ok(None);
    };
    let kind = lookahead.main.tag;

    match create_green_token_items(&lookahead, kind, state, cache, annotations)? {
        Some(node) => {
            let annotation = Annotation { node_type: NodeType::Error, state, recovery: Some(Recovery::Delete) };
            let status = AnnotationStatus{ 
                kind,
                range_from: lookahead.offset_start(), 
                len: lookahead.token_len(), 
            };

            let id = next_node_id();
            annotations.insert(id, (annotation, status));

            Ok(Some((id, node)))
        }
        None => Ok(None)
    }
}

fn create_blank_error_node(lookahead_offset: usize, current_state: usize, cache: &mut NodeCache<InternCache>, annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<Option<NodeElement>, anyhow::Error> {
    let brank_token = Token {
        leading: None,
        main: TokenItem { tag: syntax_kind::r#SPACE, offset: lookahead_offset, len: 0, value: None },
        trailing: None,
    };
    let kind = syntax_kind::r#ILLEGAL;

    match create_green_token_items(&brank_token, kind, current_state, cache, annotations)? {
        Some(node) => {
            let annotation = Annotation { node_type: NodeType::Error, state: current_state, recovery: Some(Recovery::Shift) };
            let status = AnnotationStatus{ 
                kind,
                range_from: brank_token.offset_start(), 
                len: brank_token.token_len(), 
            };

            let id = next_node_id();
            annotations.insert(id, (annotation, status));

            Ok(Some(node))
        }
        None => Ok(None)
    }
}

fn create_fatal_error_node(scanner: &mut Scanner, kind: SyntaxKind, state: usize, element_stack: &mut Vec<Option<NodeElementOrError>>, cache: &mut NodeCache<InternCache>, annotation_map: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<(NodeId, NodeElement), anyhow::Error> {
    let (mut offset, mut len) = element_stack.iter()
        .filter_map(|x| match x {
            Some(NodeElementOrError::Element { id, .. }) | Some(NodeElementOrError::Error { id, .. }) => annotation_map.get(id),
            None => None,
        })
        .fold((usize::MAX, 0), |(acc_offset, acc_len), (_, status)| {
            (usize::min(status.range_from, acc_offset), acc_len + status.len)
        })
    ;

    let mut children = element_stack.drain(..)
        .filter_map(|x| match x {
            Some(NodeElementOrError::Element { element, .. }) | Some(NodeElementOrError::Error { element, .. }) => Some(element),
            None => None,
        })
        .collect::<Vec<_>>()
    ;
    while let Some(lookahead) = scanner.shift() {
        offset = usize::min(lookahead.offset_start(), offset);
        len += lookahead.token_len();

        if let Some(child) = create_green_token_items(&lookahead, kind, state, cache, annotation_map)? {
            let annotation = Annotation { node_type: NodeType::Error, state, recovery: None };
            let status = AnnotationStatus{ 
                kind,
                range_from: lookahead.offset_start(), 
                len: lookahead.token_len(), 
            };

            let id = next_node_id();
            annotation_map.insert(id, (annotation, status));

            children.push(child);
        }
        if lookahead.main.tag == syntax_kind::SEMI { break }
    }

    let (id, node) = create_fatal_error_node_internal(children, kind, offset, len, state, annotation_map);

    Ok((id, node))
}

fn create_fatal_error_node_internal(children: Vec<NodeOrToken<GreenNode, GreenToken>>, kind: SyntaxKind, offset: usize, len: usize, state: usize, annotation_map: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> (NodeId, NodeElement) {
    let node = cstree::green::GreenNode::new(kind.into_raw(), children);
    let id = next_node_id();

    let annotation = Annotation { node_type: crate::NodeType::FatalError, state, recovery: None };
    let staus = AnnotationStatus{ kind, range_from: offset, len };

    annotation_map.insert(id, (annotation, staus));

    (id, NodeElement::Node(node))
}

fn replay_delete_recovery(
    error_events: &[TransitionEvent],
    events: &[TransitionEvent], 
    scanner: &mut Scanner, 
    state_stack: &mut StateStack, 
    element_stack: &mut Vec<Option<NodeElementOrError>>,
    node_annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>, 
    cache: &mut NodeCache<InternCache>) -> Result<Option<NodeGenerated>, anyhow::Error> 
{
    for event in error_events {
        let error = create_drop_error_node(scanner.shift(), event.current_state(), cache, node_annotations)?;
        element_stack.push(error.map(|(id, element)| NodeElementOrError::into_error(id, element)));
    }

    let result =replay_translation_event(events, NodeType::TokenSet, None, scanner, state_stack, element_stack, node_annotations, cache)?;
    if result.is_some() {
        return Ok(result);
    }

    match element_stack.pop() {
        Some(Some(node)) => {
            Ok(Some(NodeGenerated::Node(Some((node.kind(), node)))))
        }
        Some(None) => Ok(Some(NodeGenerated::Node(None))),
        None => Ok(None)
    }
}

fn replay_shift_recovery(
    error_events: &[TransitionEvent],
    events: &[TransitionEvent], 
    scanner: &mut Scanner, 
    state_stack: &mut StateStack, 
    element_stack: &mut Vec<Option<NodeElementOrError>>,
    node_annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>, 
    cache: &mut NodeCache<InternCache>) -> Result<Option<NodeGenerated>, anyhow::Error> 
{
    replay_translation_event(&error_events, NodeType::Error, Some(Recovery::Shift), scanner, state_stack, element_stack, node_annotations, cache)?;

    let result = replay_translation_event(events, NodeType::TokenSet, None, scanner, state_stack, element_stack, node_annotations, cache)?;
    if result.is_some() {
        return Ok(result);
    }

    match element_stack.pop() {
        Some(Some(node)) => {
            Ok(Some(NodeGenerated::Node(Some((node.kind(), node)))))
        }
        Some(None) => {
            Ok(Some(NodeGenerated::Node(None)))
        }
        None => Ok(None)
    }
}

fn replay_translation_event(
    events: &[TransitionEvent], 
    node_type: NodeType, 
    recovery_type: Option<Recovery>,
    scanner: &mut Scanner, 
    state_stack: &mut StateStack, 
    element_stack: &mut Vec<Option<NodeElementOrError>>,
    node_annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>, 
    cache: &mut NodeCache<InternCache>) -> Result<Option<NodeGenerated>, anyhow::Error> 
{
    for event in events {
        match event {
            TransitionEvent::Shift { syntax_kind, current_state, next_state } => {
                let current_state = state_stack.mark_checkpoint(*current_state);
                state_stack.push(*next_state);

                let id = match recovery_type {
                    Some(Recovery::Shift) => {
                        let token = scanner.lookahead().unwrap();
                        create_blank_error_node(token.offset_start(), current_state, cache, node_annotations)?
                        .map(|node| {
                            let kind = SyntaxKind::from_raw(node.kind());
                            let annotation = Annotation { node_type: node_type.clone(), state: current_state, recovery: recovery_type.clone() };
                            let status = AnnotationStatus { kind, range_from: token.offset_start(), len: 0 };
                            let id = next_node_id();
                        
                            node_annotations.insert(id, (annotation, status));
                            element_stack.push(Some(NodeElementOrError::into_element(id, node)));
                            id
                        })
                    }
                    _ => {
                        let token = scanner.shift().unwrap();
                        create_green_token_items(&token, *syntax_kind, current_state, cache, node_annotations)?
                        .map(|node| {
                            let annotation = Annotation { node_type: node_type.clone(), state: current_state, recovery: recovery_type.clone() };
                            let status = AnnotationStatus::new(&token);
                            let id = next_node_id();
                        
                            node_annotations.insert(id, (annotation, status));
                            element_stack.push(Some(NodeElementOrError::into_element(id, node)));
                            id
                        })
                    }
                };
                
                eprintln!("[DEBUG] Shift/kind: {}, state: {}, id: {:?}, push ({:?})", syntax_kind.text, current_state, id, state_stack.values());
            }
            TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count } => {
                state_stack.pop_n(*pop_count);
                let current_state = if *pop_count > 0 { state_stack.resolve_checkpoint(*pop_count) } else { state_stack.mark_checkpoint(*current_state) };
                state_stack.push(*next_state);

                let node = create_green_node(*syntax_kind, current_state, *pop_count, element_stack, node_annotations)?;
                let id = node.as_ref().map(|(id, _)| id.clone());

                element_stack.push(node.map(|(id, element)| NodeElementOrError::into_element(id, element)));
                eprintln!("[DEBUG] Reduce/kind: {}, state: {}, id: {:?}, pop({})&push ({:?})", syntax_kind.text, current_state, id, pop_count, state_stack.values());
            }
            TransitionEvent::Accept { syntax_kind, current_state } => {
                let root = create_green_node(*syntax_kind, *current_state, element_stack.len(), element_stack, node_annotations)?
                    .and_then(|(_, element)| NodeElement::into_node(element))
                    .unwrap()
                ;

                return Ok(Some(NodeGenerated::Root(*syntax_kind, next_node_id(), root)));
            }
            TransitionEvent::Error { .. } => {
                return Ok(None)
            }
        }
    }

    Ok(None)
}

fn create_syntax_tree(root: GreenNode, id: NodeId, intern_cache: InternCache, node_annotations: HashMap<NodeId, (Annotation, AnnotationStatus)>, resolve_rules: &HashMap<(SyntaxKind, SyntaxKind), SyntaxKind>, language: &Language) -> SyntaxTree {
    let mut annotations = node_annotations.into_iter()
        .map(|(id, (annotation, status))| {
            let key = AnnotationKey{ kind: status.kind, offset: status.range_from, len: status.len, is_node: annotation.is_node() };
            (key, (id, annotation))
        })
        .collect::<HashMap<_, _>>()
    ;
    let root = resolve_anotation_status(SyntaxNode::new_root(root), &mut annotations, &resolve_rules);
    
    let red_root = SyntaxNode::new_root_with_resolver(root, intern_cache.clone());
    
    let key = AnnotationKey::from(red_root.syntax());
    let annotation = Annotation { node_type: crate::NodeType::Node, state: 0, recovery: None };
    annotations.insert(key, (id, annotation));

    SyntaxTree::new(red_root, language.clone(), intern_cache.clone(), annotations)
}

#[derive(Eq, Clone, Debug)]
struct AnnotationStatus {
    kind: SyntaxKind,
    range_from: usize,
    len: usize,
}

impl AnnotationStatus {
    pub fn new(token: &Token) -> Self {
        Self {
            kind: token.main.tag,
            range_from: token.offset_start(),
            len: token.token_len(),    
        }
    }
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
    #[allow(unused)]
    recovery: Recovery,
    events: Vec<TransitionEvent>,
}

impl Journal {
    pub fn score(&self) -> usize {
        self.events.iter()
        .filter(|event| if let TransitionEvent::Shift { .. } = event { true } else { false })
        .count()
    }
}

fn try_state_recovery_by_drop(scanner: &mut Scanner, state_stack: &StateStack, failed_state: usize, penalty: &RecoveryPenalty, language: &Language) -> Result<Option<(Journal, Journal)>, anyhow::Error> {
    if penalty.delete_slot == 0 { return Ok(None); }

    let scope = scanner.scope();
    // drop lookahead
    let mut error_events = vec![];

    while let Some(lookahead) = scanner.shift() {
        let mut state_stack = state_stack.clone();
        
        match parse_state(Some(&lookahead.main.tag), failed_state, &mut state_stack, language, false)? {
            TransitionEvent::Error { .. } => {
                error_events.push(TransitionEvent::Shift { syntax_kind: lookahead.main.tag, current_state: failed_state, next_state: failed_state })
            }
            _ => { break },
        }
        if error_events.len() == penalty.delete_slot { 
            scanner.revert(scope);
            return Ok(None); 
        }
    }

    let journals = try_state_recovery_internal(scanner, &state_stack, language)
        .map(|events| {(
            Journal { events: error_events, recovery: Recovery::Delete },
            Journal { events, recovery: Recovery::Delete }
        )})
    ;
    scanner.revert(scope);
    
    Ok(journals)
}

#[derive(Clone)]
struct ShiftRecoveryItem {
    state_stack: StateStack,
    next_state: usize,
    kind: SyntaxKind,
    index: usize,
    parent: Option<Rc<ShiftRecoveryItem>>,
    event: TransitionEvent,
}

fn try_state_recovery_by_shift(scanner: &mut Scanner, state_stack: &StateStack, failed_state: usize, penalty: &RecoveryPenalty, language: &Language) -> Result<Option<(Journal, Journal)>, anyhow::Error> {
    let scope = scanner.scope();

    let state_stack = state_stack.clone();
    let histories = fetch_shift_candidates(&state_stack, failed_state, None, 0, penalty.shift_limit, language);

    let Some((error_journal, journal)) = try_state_recovery_by_shift_internal_ph1(histories, scanner, language, penalty.shift_limit - penalty.shift_decay)? else {
        scanner.revert(scope);
        return Ok(None);
    };

    scanner.revert(scope);
    
    Ok(Some((error_journal, journal)))
}

fn fetch_shift_candidates(state_stack: &StateStack, current_state: usize, parent: Option<Rc<ShiftRecoveryItem>>, depth: usize, sampling_rate: usize, language: &Language) -> Vec<Rc<ShiftRecoveryItem>> {
    let (shift_actions, reduce_actions): (Vec<_>, Vec<_>) = language.fetch_state_actions(current_state).into_iter()
        .partition(|(_, transition)| match transition {
            LookaheadTransition::Shift { .. } => true,
            _ => false,
        })
    ;
    let sampling_size = usize::max((shift_actions.len() + reduce_actions.len()) >> sampling_rate , 1);
    
    let mut items = Vec::with_capacity(sampling_size * 2);
    // Sampling shift actions
    fetch_shift_candidates_internal(&shift_actions, state_stack, parent.clone(), depth, sampling_size, language, &mut items);
    // Sampling reduce actions
    fetch_shift_candidates_internal(&reduce_actions, state_stack, parent.clone(), depth, sampling_size * 2 - items.len(), language, &mut items);

    items
}

fn fetch_shift_candidates_internal(actions: &[(&u32, &LookaheadTransition)], state_stack: &StateStack, parent: Option<Rc<ShiftRecoveryItem>>, depth: usize, sampling_size: usize, language: &Language, items: &mut Vec<Rc<ShiftRecoveryItem>>) {
    let Some(current_state) = state_stack.peek() else {
        return;
    };

    let mut used_ids = std::collections::HashSet::new();
    let mut reduce_kind = std::collections::HashSet::new();

    let iter = actions.into_iter()
        .filter(|(id, _)| used_ids.insert(*id))
        .map(|(id, _)| id)
        .filter_map(|&id| {
            let mut state_stack = state_stack.clone();
            use cstree::Syntax;
            let kind = SyntaxKind::from_raw(cstree::RawSyntaxKind(*id));
            match parse_state(Some(&kind), *current_state, &mut state_stack, language, false) {
                Ok(TransitionEvent::Shift { syntax_kind, current_state: state, next_state }) => {
                    Some(Rc::new(ShiftRecoveryItem{ 
                        state_stack,
                        next_state,
                        kind: syntax_kind.clone(),
                        index: depth,
                        parent: parent.clone(), 
                        event: TransitionEvent::Shift { syntax_kind, current_state: state, next_state }, 
                    }))
                }
                Ok(TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count }) if reduce_kind.insert(syntax_kind) => {    
                    Some(Rc::new(ShiftRecoveryItem{ 
                        state_stack,
                        next_state,
                        kind: syntax_kind.clone(),
                        index: depth,
                        parent: parent.clone(), 
                        event: TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count }, 
                    }))
                }
                _ => None,
            }
        })
        .take(sampling_size)
    ;

    items.extend(iter);

}

fn try_state_recovery_by_shift_internal_ph1(
    mut histories: Vec<Rc<ShiftRecoveryItem>>,
    scanner: &mut Scanner,
    language: &Language,
    max_depth: usize) -> Result<Option<(Journal, Journal)>, anyhow::Error>
{
    let mut best_history = None;
    let mut best_events = None;

    let mut depth = 0;

    let Some(lookahead) = scanner.lookahead().cloned() else {
        return Ok(None)
    };

    while depth < max_depth {
        let mut next_histories = vec![];

        for history in histories {
            match (lookahead.main.tag == history.kind, history.parent.as_ref()) {
                (true, Some(parent)) => {
                    // to ph2
                    let scope = scanner.scope();
                    let candidate = try_state_recovery_by_shift_internal_ph2(scanner, parent.clone(), language);
                    if judge_shift_recover_candidate(best_events.as_ref(), candidate.as_ref()) {
                        best_history = history.parent.clone();
                        best_events = candidate;
                    }
                    scanner.revert(scope);
                }
                _ => {
                    next_histories.extend(fetch_shift_candidates(&history.state_stack, history.next_state, Some(history.clone()), depth + 1, max_depth - depth - 1, language));
                }
            }
        }

        depth += 1;
        histories = next_histories;
    }

    let best_error_journal = journal_from_shift_history(best_history, max_depth);

    Ok(best_error_journal.zip(best_events))
}

fn try_state_recovery_by_shift_internal_ph2(scanner: &mut Scanner, history: Rc<ShiftRecoveryItem>, language: &Language) -> Option<Journal> {
    let mut state_stack = history.state_stack.clone();
    try_state_recovery_internal(scanner, &mut state_stack, language)
    .map(|events| Journal { events, recovery: Recovery::Shift })
}

fn judge_shift_recover_candidate(lhs: Option<&Journal>, rhs: Option<&Journal>) -> bool {
    match (lhs, rhs) {
        (Some(l_journal), Some(r_journal)) if l_journal.score() < r_journal.score() => true,
        (None, Some(_)) => true,
        _ => false
    }
}

fn journal_from_shift_history(mut history: Option<Rc<ShiftRecoveryItem>>, max_depth: usize) -> Option<Journal> {
    let mut error_events = Vec::with_capacity(max_depth);
    error_events.resize(max_depth, None);

    while let Some(h) = history {
        error_events[h.index] = Some(h.event.clone());
        history = h.parent.clone();
    }

    Some(Journal { 
        events: error_events.into_iter().filter_map(|event| event).collect(), 
        recovery: Recovery::Shift 
    })
}

fn try_state_recovery_internal(scanner: &mut Scanner, state_stack: &StateStack, language: &Language) -> Option<Vec<TransitionEvent>> {
    let mut events = Vec::with_capacity(64);
    let mut state_stack = state_stack.clone();
    
    while let Some(lookahead) = scanner.lookahead() {
        if (lookahead.main.tag == syntax_kind::r#SEMI) || (lookahead.main.tag == syntax_kind::r#EOF) {
            break
        }
        let current_state = state_stack.peek().unwrap();
    
        match parse_state(Some(&lookahead.main.tag), *current_state, &mut state_stack, language, false) {
            Ok(TransitionEvent::Error { .. }) => return None,
            Err(_) => return None,
            Ok(TransitionEvent::Shift { syntax_kind, current_state, next_state }) => {
                scanner.shift();
                events.push(TransitionEvent::Shift { syntax_kind, current_state, next_state });
            }
            Ok(TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count, .. }) if pop_count == 0 => {
                events.push(TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count });
            }
            Ok(TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count, .. }) => {
                events.push(TransitionEvent::Reduce { syntax_kind, current_state, next_state, pop_count });
                break;
            }
            Ok(TransitionEvent::Accept { syntax_kind, current_state }) => {
                events.push(TransitionEvent::Accept { current_state, syntax_kind });
                break;
            }
        }
    }

    Some(events)
}

#[derive(Debug)]
pub struct EditScope {
    pub offset: u32,
    pub from_len: u32,
    pub to_len: u32,
}

enum IncrementalNodeGenerated {
    Success { id: NodeId, node: NodeElement },
    FatalError { id: NodeId, node: NodeElement },
}

pub struct IncrementalParser {
    tree: SyntaxTree,
    edit_node: SyntaxNode<SyntaxKind>,
}

impl IncrementalParser {
    pub fn create(tree: &SyntaxTree, edit: EditScope) -> Result<Self, anyhow::Error> {
        let edit_node = match find_edit_node(tree, &edit) {
            Some(node) => {
                node
            }
            None => {
                bail!("Can not find incrementa parse target node ({:?})", edit);
            }
        };

        Ok(Self {
            tree: tree.clone(),
            edit_node,
        })
    }

    pub fn parse(&self, source: String) -> Result<SyntaxTree, anyhow::Error> {
        let Some(metadata) = self.tree.get_annotation_of(AnnotationKey::from(&self.edit_node)) else {
            bail!("Invalid state of edit node (kind: {})", self.edit_node.kind().text);
        };

        let mut state_stack = StateStack::new(metadata.state);
        let mut scanner = Scanner::create(source.to_string(), self.edit_node.text_range().start().into());
        let mut intern_cache = self.tree.intern_cache.clone();
        let mut cache = NodeCache::with_interner(&mut intern_cache);
        let mut node_annotations = HashMap::new();

        let mut penalty = RecoveryPenalty { 
            delete_slot: 3, 
            shift_limit: 9,
            shift_decay: 0, 
            next_shift_decay: 2, 
        };

        let (red_node, new_annotations) = match incremental_parse(&mut scanner, &mut state_stack, self.edit_node.kind(), &mut node_annotations, &mut cache, &mut penalty, &self.tree.language)? {
            IncrementalNodeGenerated::Success { id, node: new_node } => {
                let Some(parent) = self.edit_node.parent() else {
                    bail!("Need parent for edit node (kind: {})", self.edit_node.kind().text);
                };
                let Some(index) = parent.children().enumerate().find(|(_, node)| **node == self.edit_node).map(|(i, _)| i) else {
                    bail!("Can not determine edit node index (kind: {})", self.edit_node.kind().text);
                };
                replace_generated_node(id, new_node, index, &self.edit_node, parent, &intern_cache, &self.tree.annotations, &mut node_annotations)?

            }
            IncrementalNodeGenerated::FatalError { id, node: new_node } => {
                let Some((index, anscestor)) = self.edit_node.root().children().enumerate().find(|(_, child)| self.edit_node.ancestors().any(|x| x == *child)) else {
                    bail!("Need anscestor node of root (kind: {})", self.edit_node.kind().text);
                };
                replace_generated_node(id, new_node, index, &anscestor, anscestor.root(), &intern_cache, &self.tree.annotations, &mut node_annotations)?
            }
        };

        Ok(SyntaxTree{ root: red_node, intern_cache, annotations: Rc::new(new_annotations), ..self.tree.clone() })
    }
}

fn replace_generated_node(
    id: NodeId, new_node: NodeElement, index: usize, 
    old_node: &SyntaxNode<SyntaxKind>,
    parent: &SyntaxNode<SyntaxKind>, 
    intern_cache: &InternCache,
    old_annotations: &HashMap<AnnotationKey, ((Instant, u64), Annotation)>,
    node_annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>) -> Result<(ResolvedNode<SyntaxKind>, HashMap<AnnotationKey, ((Instant, u64), Annotation)>), anyhow::Error> 
{
    let Some((_, status)) = node_annotations.get(&id) else {
        bail!("Annotation ust be created");
    };
    let kind = SyntaxKind::from_raw(new_node.kind());
          
    let mut children = parent.green().children()
        .map(|node| match node {
            NodeOrToken::Node(x) => NodeElement::Node(x.clone()),
            NodeOrToken::Token(x) => NodeElement::Token(x.clone()),
        }).collect::<Vec<_>>()
    ;

    children.splice(index..=index, vec![new_node.clone()]);
    let new_parent = GreenNode::new(parent.syntax_kind(), children);

    let new_root = parent.replace_with(new_parent);
    let red_node = SyntaxNode::new_root_with_resolver(new_root, intern_cache.clone());

    let merge_set_before = partition_node_id_with_dirty(&parent.root(), AnnotationKey::from(old_node), NodeMergeSet::as_merge_set);
    let key = AnnotationKey { kind, offset: status.range_from, len: status.len,is_node: true };
    let merge_set_after = partition_node_id_with_dirty(red_node.syntax(), key, NodeMergeSet::as_merge_set);

    let mut new_annotations = merge_set_before.into_iter().zip(merge_set_after)
        .filter_map(|(before, after)| match (before, after) {
            (NodeMergeSet::Unmodified(old_key), NodeMergeSet::Unmodified(new_key)) => {
                let Some((id, annotation)) = old_annotations.get(&old_key) else { return None; };
                Some((new_key.clone(), (id.clone(), annotation.clone())))
            }
            _ => None,
        })
        .collect::<HashMap<_, _>>()
    ;
    new_annotations.extend(
        node_annotations.into_iter()
        .map(|(id, (annotation, status))| (
            AnnotationKey { kind: status.kind, offset: status.range_from, len: status.len, is_node: annotation.is_node() },
            (id.clone(), annotation.clone())
        ))
    );

    Ok((red_node, new_annotations))
}

fn find_edit_node(tree: &SyntaxTree, edit: &EditScope) -> Option<SyntaxNode<SyntaxKind>> {
    let lower_at = TextSize::from(edit.offset);
    let upper_at = TextSize::from(edit.offset.saturating_add(edit.from_len));

    let root = tree.root();
    let Some(root_node) = root.as_inner_node() else {
        return None;
    };

    let Some(lower_node) = find_deepest_token_containing(root_node, lower_at) else { return None; };
    let Some(upper_node) = find_deepest_token_containing(root_node, upper_at) else { return None; };

    let lower_path = lower_node.prev_token().unwrap_or(lower_node).ancestors().collect::<Vec<_>>().into_iter().rev();
    let upper_path = upper_node.next_token().unwrap_or(upper_node).ancestors().collect::<Vec<_>>().into_iter().rev();
    
    let mut common_parent = None;
    
    for (lower, upper) in lower_path.zip(upper_path) {
        if lower != upper { break }
        common_parent = Some(lower);
    }

    // FIXME: range over statements

    common_parent.cloned()
}

fn find_deepest_token_containing(root: &SyntaxNode<SyntaxKind>, needle: TextSize) -> Option<&SyntaxToken<SyntaxKind>> {
    let mut element = root;

    loop {
        let child = element.children_with_tokens()
            .find(|x| {
                match x.text_range() {
                    child_range if child_range.len() == cstree::text::TextSize::from(0) => child_range.start() == needle,
                    child_range => child_range.contains(needle)
                }
            })
        ;

        match child {
            Some(NodeOrToken::Token(token)) => {
                return Some(token);
            }
            Some(NodeOrToken::Node(node)) => {
                element = node;
            }
            None => { break }
        }
    }
    
    None
}

fn incremental_parse(scanner: &mut Scanner, state_stack: &mut StateStack, terminate_kind: SyntaxKind, node_annotations: &mut HashMap<NodeId, (Annotation, AnnotationStatus)>, cache: &mut NodeCache<InternCache>, penalty: &mut RecoveryPenalty, language: &Language) -> Result<IncrementalNodeGenerated, anyhow::Error> {
    let mut element_stack = vec![];

    while let Some(_lookahead) = scanner.lookahead() {
        match parse_internal(scanner, state_stack, &mut element_stack, node_annotations, cache, penalty, language)? {
            NodeGenerated::Node(Some((kind, element))) if kind == terminate_kind => {
                return match element {
                    NodeElementOrError::Element { id, element } => {
                        Ok(IncrementalNodeGenerated::Success {id, node: element })
                    }
                    NodeElementOrError::Error { id, element } => {
                        Ok(IncrementalNodeGenerated::Success {id, node: element })
                    }
                };
            }
            NodeGenerated::Node(Some((_, element))) => {
                element_stack.push(Some(element));
            }
            NodeGenerated::Node(None) => {
                element_stack.push(None);
            }
            NodeGenerated::Root(kind, id, element) if kind == terminate_kind => {
                return Ok(IncrementalNodeGenerated::Success {id, node: NodeElement::Node(element) });
            }
            NodeGenerated::RootMember(_) => {
                todo!()
            }
            NodeGenerated::Fatal(id, element) => {
                return Ok(IncrementalNodeGenerated::FatalError { id, node: element });
            }
            _ => {}
        }
    }
    
    let (offset, len) = element_stack.iter()
        .filter_map(|x| match x {
            Some(NodeElementOrError::Element { id, .. }) | Some(NodeElementOrError::Error { id, .. }) => node_annotations.get(id),
            None => None,
        })
        .fold((usize::MAX, 0), |(acc_offset, acc_len), (_, status)| {
            (usize::min(status.range_from, acc_offset), acc_len + status.len)
        })
    ;
    let children = element_stack.into_iter()
        .filter_map(|x| match x {
            Some(NodeElementOrError::Element { element, .. }) => Some(element),
            Some(NodeElementOrError::Error { element, .. }) => Some(element),
            None => None,
        })
        .collect::<Vec<_>>()
    ;

    let (id, fatal_node) = create_fatal_error_node_internal(children, syntax_kind::r#ILLEGAL, offset, len, 0, node_annotations);
    Ok(IncrementalNodeGenerated::FatalError { id, node: fatal_node })
}

pub enum NodeMergeSet {
    Unmodified(AnnotationKey),
    Modified
}

impl NodeMergeSet {
    pub fn as_merge_set(key: Option<AnnotationKey>, dirty: bool) -> Option<Self> {
        match (key, dirty) {
            (Some(key), false) => {
                Some(NodeMergeSet::Unmodified(key.clone()))
            }
            (_, true) => {
                Some(NodeMergeSet::Modified)
            }
            _ => {
                None
            }
        }
    }
}

fn partition_node_id_with_dirty<T>(root: &SyntaxNode<SyntaxKind>, needle: AnnotationKey, factory: impl Fn(Option<AnnotationKey>, bool) -> Option<T>) -> Vec<T> {
    root.preorder_with_tokens()
        .scan(false, |dirty_zone, event| {
            match event {
                cstree::traversal::WalkEvent::Enter(NodeOrToken::Node(node)) if (AnnotationKey::from(node) == needle) && (! *dirty_zone) => {
                    *dirty_zone = true;
                    Some(factory(None, *dirty_zone))
                },
                cstree::traversal::WalkEvent::Enter(_) if *dirty_zone => {
                    Some(None)
                }
                cstree::traversal::WalkEvent::Enter(NodeOrToken::Node(node)) => {
                    Some(factory(Some(AnnotationKey::from(node)), *dirty_zone))
                },
                cstree::traversal::WalkEvent::Enter(NodeOrToken::Token(node)) => {
                    Some(factory(Some(AnnotationKey::from(node)), *dirty_zone))
                },
                cstree::traversal::WalkEvent::Leave(NodeOrToken::Node(node)) if AnnotationKey::from(node) == needle => {
                    *dirty_zone = false;
                    Some(None)
                }
                cstree::traversal::WalkEvent::Leave(_) => {
                    Some(None)
                }
            }
            
        })
        .filter_map(std::convert::identity)
        .collect::<Vec<_>>()
}
