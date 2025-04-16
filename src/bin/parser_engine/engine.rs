use syntax_kind::maps::SYNTAX_KIND_MAP;

pub mod syntax_kind;
mod transition_state;
mod scan_rule;

pub use transition_state::{init_lookahead_translations, eof_transition, init_goto_transition_table};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
pub struct SyntaxKind {
    pub id: u32,
    pub text: &'static str,
    pub is_keyword: bool,
    pub is_terminal: bool,
}

impl cstree::Syntax for SyntaxKind {
    fn from_raw(raw: cstree::RawSyntaxKind) -> Self {
        *SYNTAX_KIND_MAP.get(&raw.0).unwrap()
    }

    fn into_raw(self) -> cstree::RawSyntaxKind {
        cstree::RawSyntaxKind(self.id)
    }

    fn static_text(self) -> Option<&'static str> {
        if self.is_keyword {
            return Some(self.text);
        }
        None
    }
}

#[derive(Clone)]
pub enum LookaheadTransition {
    Unknown,
    Shift { next_state: usize },
    Reduce{ pop_count: usize, lhs: u32 },
    Accept{ last_state: usize, last_kind: SyntaxKind },
}

impl Default for LookaheadTransition {
    fn default() -> Self {
        LookaheadTransition::Unknown
    }
}

pub enum TransitionEvent {
    Shift { syntax_kind: SyntaxKind, current_state: usize, next_state: usize, input: Option<String> },
    Reduce{ syntax_kind: SyntaxKind, current_state: usize, next_state: usize, pop_count: usize },
    Error { syntax_kind: SyntaxKind, failed_state: usize, pop_count: usize, candidate_syntax_kinds: Vec<SyntaxKind> },
    Accept{ current_state: usize, syntax_kind: SyntaxKind },
}

