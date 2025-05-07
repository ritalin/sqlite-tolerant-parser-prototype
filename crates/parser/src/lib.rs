use std::{collections::HashMap, rc::Rc};

use cstree::{green::{GreenNode, GreenToken}, interning::{InternKey, TokenKey}, syntax::SyntaxToken, text::TextRange, util::NodeOrToken};
use sqlite_parser_proto::{engine, LookaheadTransition, SyntaxKind};

mod parser;
pub use parser::{Parser, AnnotationKey, NodeId, EditScope, IncrementalParser};

type NodeElement = NodeOrToken::<GreenNode, GreenToken>;

#[derive(Clone, Default)]
pub struct Language;

impl Language {
    pub fn resolve_lookahead_state(&self, lookahead: Option<&SyntaxKind>, current_state: usize) -> Result<LookaheadTransition, anyhow::Error> {
        match lookahead {
            Some(kind) => {
                engine::resolve_parser_next_state(current_state, kind)
            }
            None => {
                engine::resolve_parser_accept_state(current_state)
            }
        }
    }

    pub fn resolve_goto_state(&self, state: usize, kind_id: u32) -> Result<usize, anyhow::Error> {
        engine::resolve_parser_goto_state(state, kind_id)
    }

    pub fn fetch_state_actions(&self, state: usize) -> Vec<(&'static u32, &'static LookaheadTransition)> {
        engine::fetch_state_actions(state)
    }
}

#[derive(Clone)]
pub struct SyntaxTree {
    root: cstree::syntax::ResolvedNode<SyntaxKind>,
    language: Language,
    intern_cache: InternCache,
    pub annotations: Rc<HashMap<AnnotationKey, (parser::NodeId, Annotation)>>,
}

impl SyntaxTree {
    pub fn new(root: cstree::syntax::ResolvedNode<SyntaxKind>, language: Language, intern_cache: InternCache, annotations: HashMap<AnnotationKey, (parser::NodeId, Annotation)>) -> Self {
        Self {
            root,
            language,
            intern_cache,
            annotations: Rc::new(annotations),
        }
    }

    pub fn root(&self) -> self::SyntaxNode {
        self::SyntaxNode::from_node(self.root.syntax(), self.annotations.clone())
    }

    pub fn language(&self) -> &Language {
        &self.language
    }

    pub fn debug(&self, recursive: bool) -> String {
        self.root.debug(&self.intern_cache, recursive)
    }

    pub fn display(&self) -> String {
        self.root.display(&self.intern_cache)
    }

    pub fn get_annotation_of(&self, key: AnnotationKey) -> Option<&Annotation> {
        self.annotations.get(&key).map(|(_, annotation)| annotation)
    }

    pub fn covering_element(&self, range: cstree::text::TextRange) -> Option<SyntaxNode> {
        let mut element = &self.root;

        loop {
            let child = element.children_with_tokens()
                .find(|x| {
                    match x.text_range() {
                        child_range if child_range.len() == cstree::text::TextSize::from(0) => child_range.start() == range.start(),
                        child_range => child_range.contains(range.start())
                    }
                })
            ;

            match child {
                Some(NodeOrToken::Token(token)) => {
                    return Some(SyntaxNode::from_node(token.parent(), self.annotations.clone()));
                }
                Some(NodeOrToken::Node(node)) if node.arity_with_tokens() == 0 => {
                    return Some(SyntaxNode::from_node(node, self.annotations.clone()));
                }
                Some(NodeOrToken::Node(node)) => {
                    element = node;
                }
                None => { break }
            }
        }
        
        None

    //     let iter = self.root().preorder()
    //     .filter_map(|event| match event {
    //         cstree::traversal::WalkEvent::Enter(node) => Some(node),
    //         cstree::traversal::WalkEvent::Leave(_) => None,
    //     });
    
    //     if range.len() == Into::into(0) {
    //         let node = self.root().preorder()
    //             .filter_map(|event| match event {
    //                 cstree::traversal::WalkEvent::Enter(node) => Some(node),
    //                 cstree::traversal::WalkEvent::Leave(_) => None,
    //             })
    //             .skip_while(|node| node.text_range().start() < range.start())
    //             .take_while(|node| node.text_range() == range)
    //             .last()
    //         ;
    //         if node.is_some() {
    //             return node;
    //         }
    //     }
    
    //     self.root().preorder()
    //     .filter_map(|event| match event {
    //         cstree::traversal::WalkEvent::Enter(node) => Some(node),
    //         cstree::traversal::WalkEvent::Leave(_) => None,
    //     })
    //     .take_while(|node| node.text_range().contains_range(range)).last()
    }
}

#[derive(Clone)]
pub struct SyntaxNode {
    inner_node: NodeOrToken<cstree::syntax::SyntaxNode<SyntaxKind>, SyntaxToken<SyntaxKind>>,
    metadata_map: Rc<HashMap<AnnotationKey, (NodeId, Annotation)>>,
}

impl SyntaxNode {
    pub fn new(
        element: NodeOrToken<&cstree::syntax::SyntaxNode<SyntaxKind>, &SyntaxToken<SyntaxKind>>,
        metadata_map: Rc<HashMap<AnnotationKey, (NodeId, Annotation)>>) -> Self 
    {
        let inner_node = match element {
            NodeOrToken::Node(x) => NodeOrToken::Node(x.clone()),
            NodeOrToken::Token(x) => NodeOrToken::Token(x.clone()),
        };
        Self { inner_node, metadata_map: metadata_map.clone() }
    }

    fn from_node(
        node: &cstree::syntax::SyntaxNode<SyntaxKind>, 
        metadata_map: Rc<HashMap<AnnotationKey, (NodeId, Annotation)>>) -> Self 
    {
        Self { inner_node: NodeOrToken::Node(node.clone()), metadata_map: metadata_map }
    }

    fn from_token(
        node: &SyntaxToken<SyntaxKind>, 
        metadata_map: Rc<HashMap<AnnotationKey, (NodeId, Annotation)>>) -> Self 
    {
        Self { inner_node: NodeOrToken::Token(node.clone()), metadata_map: metadata_map }
    }

    pub(crate) fn as_inner_node(&self) -> Option<&cstree::syntax::SyntaxNode<SyntaxKind>> {
        match &self.inner_node {
            NodeOrToken::Node(x) => Some(x),
            NodeOrToken::Token(_) => None,
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.inner_node.kind()
    } 
    
    pub fn metadata(&self) -> Annotation {
        Self::metadata_with_key(self.metadata_map.clone(), &self.metadata_key()).expect("Lookup metadata failed")
    }
    
    fn metadata_with_key(metadata_map: Rc<HashMap<AnnotationKey, (NodeId, Annotation)>>, key: &AnnotationKey) -> Option<Annotation> {
        metadata_map.get(&key).map(|(_, metadata)| metadata.clone())
    }

    fn metadata_key(&self) -> AnnotationKey {
        match &self.inner_node {
            NodeOrToken::Node(x) => AnnotationKey::from(x),
            NodeOrToken::Token(x) => AnnotationKey::from(x),
        }
    }

    pub fn text_range(&self) -> TextRange {
        self.inner_node.text_range()
    }

    pub fn value(&self) -> Option<String> {
        match self.metadata().node_type {
            NodeType::TokenSet | NodeType::Error | NodeType::FatalError => {
                // delegate to child node
                let Some(inner_child) = self.inner_node.as_node().and_then(|node| node.first_child()) else {
                    return None;
                };
                SyntaxNode::from_node(inner_child, self.metadata_map.clone()).value()
            },
            NodeType::LeadingToken | NodeType::TrailingToken | NodeType::MainToken => {
                self.inner_node.as_token().map(|node| node.resolved().text().to_string())
            }
            NodeType::Node => None,
        }
    }

    pub fn leading_trivia(&self) -> Vec<SyntaxNode> {
        match self.metadata().node_type {
            NodeType::TokenSet => {
                let Some(inner_node) = self.inner_node.as_node() else {
                    return vec![];
                };
                self.enumerate_trivia(inner_node, NodeType::LeadingToken)
            }
            NodeType::LeadingToken | NodeType::TrailingToken | NodeType::MainToken => {
                // delegate to parent node
                let Some(inner_parent) = self.inner_node.parent() else {
                    return vec![];
                };
                SyntaxNode::from_node(inner_parent, self.metadata_map.clone()).leading_trivia()
            }
            NodeType::Error | NodeType::FatalError => {
                // delegate to child node
                let Some(inner_child) = self.inner_node.as_node().and_then(|node| node.first_child()) else {
                    return vec![];
                };
                SyntaxNode::from_node(inner_child, self.metadata_map.clone()).leading_trivia()
            }
            NodeType::Node => vec![],
        }
    }

    pub fn trailing_trivia(&self) -> Vec<SyntaxNode> {
        match self.metadata().node_type {
            NodeType::TokenSet => {
                let Some(inner_node) = self.inner_node.as_node() else {
                    return vec![];
                };
                self.enumerate_trivia(inner_node, NodeType::TrailingToken)
            }
            NodeType::LeadingToken | NodeType::TrailingToken | NodeType::MainToken => {
                // delegate to parent node
                let Some(inner_parent) = self.inner_node.parent() else {
                    return vec![];
                };
                SyntaxNode::from_node(inner_parent, self.metadata_map.clone()).trailing_trivia()
            }
            NodeType::Error | NodeType::FatalError => {
                // delegate to child node
                let Some(inner_child) = self.inner_node.as_node().and_then(|node| node.first_child()) else {
                    return vec![];
                };
                SyntaxNode::from_node(inner_child, self.metadata_map.clone()).trailing_trivia()
            }
            NodeType::Node => vec![],
        }
    }

    fn enumerate_trivia(&self, node: &cstree::syntax::SyntaxNode<SyntaxKind>, ty: NodeType) -> Vec<SyntaxNode> {
        node.children_with_tokens()
        .filter_map(|child| match child {
            NodeOrToken::Node(_) => None,
            NodeOrToken::Token(x) => Some((x, AnnotationKey::from(x)))
        })
        .filter(|(_, key)| {
            let Some(metadata) = Self::metadata_with_key(self.metadata_map.clone(), key) else {
                return false;
            };
            metadata.node_type == ty
        })
        .map(|(child, _)| Self::from_token(child, self.metadata_map.clone()))
        .collect()
    }

    pub fn children(&self) -> SyntaxChildren {
        SyntaxChildren::new(self.as_inner_node(), &self.metadata_map)
    }
}

pub struct SyntaxChildren {
    inner: Vec<self::SyntaxNode>,
    index: usize,
}

impl SyntaxChildren {
    pub(crate) fn new(node: Option<&cstree::syntax::SyntaxNode<SyntaxKind>>, metadata_map: &Rc<HashMap<AnnotationKey, (NodeId, Annotation)>>) -> Self {
        let inner = match node {
            Some(node) => {
                node.children_with_tokens()
                .map(|child| match child {
                    NodeOrToken::Node(x) => self::SyntaxNode::from_node(x, metadata_map.clone()),
                    NodeOrToken::Token(x) => self::SyntaxNode::from_token(x, metadata_map.clone()),
                })
                .collect::<Vec<_>>()
            }
            None => vec![],
        };

        Self { inner, index: 0 }
    }
}

impl Iterator for SyntaxChildren {
    type Item = self::SyntaxNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.inner.len() {
            return None;
        }

        let result = self.inner[self.index].clone();
        self.index += 1;

        Some(result)
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum NodeType {
    TokenSet,
    LeadingToken,
    TrailingToken,
    MainToken,
    Node,
    Error,
    FatalError,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Recovery {
    Delete,
    Shift,
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub node_type: NodeType,
    pub state: usize,
    pub recovery: Option<Recovery>,
}

impl Annotation {
    pub fn is_node(&self) -> bool {
        match self.node_type {
            NodeType::TokenSet | NodeType::Node | NodeType::Error | NodeType::FatalError => true,
            NodeType::LeadingToken | NodeType::TrailingToken | NodeType::MainToken => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InternerError {
    KeySpaceExhausted,
}

impl std::fmt::Display for InternerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InternerError::KeySpaceExhausted => write!(f, "key space exhausted"),
        }
    }
}

impl std::error::Error for InternerError {}

#[derive(Clone)]
pub struct InternCache {
    map: indexmap::IndexSet<String>,
}

impl InternCache {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl cstree::interning::Interner for InternCache {
    type Error = InternerError;

    fn try_get_or_intern(&mut self, text: &str) -> Result<cstree::interning::TokenKey, Self::Error> {
        let (i, _) = self.map.insert_full(text.to_string());

        let Ok(i) = u32::try_from(i) else {
            return Err(InternerError::KeySpaceExhausted);
        };
        let Some(key) = TokenKey::try_from_u32(i) else {
            return Err(InternerError::KeySpaceExhausted);
        };
        return Ok(key);
    }
}

impl cstree::interning::Resolver<TokenKey> for InternCache {
    fn try_resolve(&self, key: TokenKey) -> Option<&str> {
        self.map.get_index(key.into_u32() as usize).map(String::as_str)
    }

}