pub mod binding;

#[derive(serde::Deserialize)]
pub struct Grammar {
    pub start: String,
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
    fn id(&self) -> usize;
}

#[derive(Eq, Ord, Clone, Debug, serde::Deserialize)]
pub struct GrammarSymbol {
    pub id: usize,
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
    fn id(&self) -> usize {
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
    pub id: usize,
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
