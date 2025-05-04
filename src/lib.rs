pub mod engine;

#[derive(serde::Deserialize)]
pub struct Grammar {
    // pub start: String,
    pub symbols: Vec<GrammarSymbol>,
    pub rules: Vec<GrammarRule>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, serde::Deserialize)]
pub enum SymbolType {
    Terminal { is_keyword: bool },
    NonTerminal,
    MultiTerminal { classes: Vec<String> },
}

pub trait SymbolRef {
    fn id(&self) -> u32;
}

#[derive(Eq, Ord, Clone, Debug, serde::Deserialize)]
pub struct GrammarSymbol {
    pub id: u32,
    pub name: String,
    #[serde(alias = "type")]
    pub symbol_type: SymbolType,
    pub precedence: Option<Precedence>,
}

impl PartialEq for GrammarSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for GrammarSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl SymbolRef for GrammarSymbol {
    fn id(&self) -> u32 {
        self.id
    }
}

#[derive(serde::Deserialize)]
pub struct GrammarRule {
    pub lhs: String,
    pub members: Vec<GrammarRuleMember>,
}

#[derive(serde::Deserialize)]
pub struct GrammarRuleMember {
    pub id: u32,
    pub sequences: Vec<Rhs>,
    pub precedence: Option<Precedence>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Rhs {
    pub token: Term,
    pub alias: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Term {
    Symbol {name: String},
    CharClass { members: Vec<String> },
}

#[derive(PartialEq, Eq, Ord, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Precedence {
    Left(i32),
    Right(i32),
    Noassoc,
}

impl Precedence {
    pub fn score(&self) -> i32 {
        match self {
            Precedence::Left(score) => *score,
            Precedence::Right(score) => *score,
            Precedence::Noassoc => 0
        }
    }
}

impl PartialOrd for Precedence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs_score = match self.score() {
            score if score > 0 => Some(score),
            _ => None
        };
        let rhs_score = match other.score(){
            score if score > 0 => Some(score),
            _ => None
        };

        rhs_score.partial_cmp(&lhs_score)
    }
}

#[derive(Debug)]
pub struct RuleId {
    pub id: u32,
}

impl RuleId {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

impl SymbolRef for RuleId {
    fn id(&self) -> u32 {
        self.id
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
pub struct SyntaxKind {
    pub id: u32,
    pub text: &'static str,
    pub is_keyword: bool,
    pub is_terminal: bool,
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

#[derive(Clone)]
pub enum TransitionEvent {
    Shift { syntax_kind: SyntaxKind, current_state: usize, next_state: usize },
    Reduce{ syntax_kind: SyntaxKind, current_state: usize, next_state: usize, pop_count: usize },
    Error { syntax_kind: Option<SyntaxKind>, failed_state: usize },
    Accept{ current_state: usize, syntax_kind: SyntaxKind },
}

impl TransitionEvent {
    pub fn current_state(&self) -> usize {
        match self {
            TransitionEvent::Shift { current_state, .. } => *current_state,
            TransitionEvent::Reduce { current_state, .. } => *current_state,
            TransitionEvent::Error { failed_state, .. } => *failed_state,
            TransitionEvent::Accept { current_state, .. } => *current_state,
        }
    }

    pub fn next_state(&self) -> Option<usize> {
        match self {
            TransitionEvent::Shift { next_state, .. } => Some(*next_state),
            TransitionEvent::Reduce { next_state, .. } => Some(*next_state),
            TransitionEvent::Error { .. } => None,
            TransitionEvent::Accept { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScanPattern {
    pub id: u32,
    pub pattern: &'static str,
    pub len: usize,
}
