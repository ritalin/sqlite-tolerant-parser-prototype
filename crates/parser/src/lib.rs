use cstree::interning::{InternKey, TokenKey};
use sqlite_parser_proto::{engine, LookaheadTransition, SyntaxKind};

pub type SyntaxNode = cstree::syntax::ResolvedNode<SyntaxKind>;

#[derive(Clone, Default)]
pub struct Language;

impl Language {
    pub fn resolve_lookahead_state(&self, lookahead: Option<&(SyntaxKind, Option<String>)>, current_state: usize) -> Result<LookaheadTransition, anyhow::Error> {
        match lookahead {
            Some((kind, _)) => {
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
}


pub struct SyntaxTree {
    root: SyntaxNode,
    language: Language,
    intern_cache: InternCache,
}

impl SyntaxTree {
    pub fn new(root: cstree::green::GreenNode, language: Language, intern_cache: InternCache) -> Self {
        Self {
            root: SyntaxNode::new_root_with_resolver(root, intern_cache.clone()),
            language,
            intern_cache,
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