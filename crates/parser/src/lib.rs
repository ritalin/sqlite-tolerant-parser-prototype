use std::collections::HashMap;

use cstree::{green::{GreenNode, GreenToken}, interning::{InternKey, TokenKey}, syntax::ResolvedElementRef, util::NodeOrToken};
use sqlite_parser_proto::{engine, LookaheadTransition, SyntaxKind};

mod parser;
pub use parser::{Parser, AnnotationKey, NodeId, EditScope};

pub type SyntaxNode = cstree::syntax::ResolvedNode<SyntaxKind>;

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
    root: SyntaxNode,
    language: Language,
    intern_cache: InternCache,
    pub annotations: HashMap<AnnotationKey, (parser::NodeId, Annotation)>,
}

impl SyntaxTree {
    pub fn new(root: SyntaxNode, language: Language, intern_cache: InternCache, annotations: HashMap<AnnotationKey, (parser::NodeId, Annotation)>) -> Self {
        Self {
            root,
            language,
            intern_cache,
            annotations,
        }
    }

    pub fn root(&self) -> &SyntaxNode {
        &self.root
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

    pub fn covering_element(&self, range: cstree::text::TextRange) -> Option<&SyntaxNode> {
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
                    return Some(token.parent());
                }
                Some(NodeOrToken::Node(node)) if node.arity_with_tokens() == 0 => {
                    return Some(node);
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