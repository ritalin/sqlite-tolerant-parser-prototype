use anyhow::bail;
use cstree::Syntax;

use crate::{LookaheadTransition, SyntaxKind};


#[cfg(feature = "parser_generated")]
pub mod kinds { 
    use crate::SyntaxKind;
    include!("assets/generated/syntax_kind.rs"); 
}
#[cfg(not(feature = "parser_generated"))]
pub mod kinds { 
    use crate::SyntaxKind;

    pub static r#ILLEGAL: SyntaxKind = SyntaxKind{id: u32::MAX, text: "ILLEGAL", is_keyword: false, is_terminal: false};
    pub static r#EOF: SyntaxKind = SyntaxKind{id: u32::MAX, text: "EOF", is_keyword: false, is_terminal: true};
    pub static SYNTAX_KIND_MAP: phf::Map<u32, SyntaxKind> = phf::phf_map!{};
}

#[cfg(feature = "parser_generated")]
mod states {
    use crate::LookaheadTransition;
    include!("assets/generated/parser_state.rs");
}
#[cfg(not(feature = "parser_generated"))]
mod states {
    use crate::LookaheadTransition;

    pub static LA_TRANSITION_TABLE: &[phf::Map<u32, LookaheadTransition>] = &[];
    pub static GOTO_TRANSITION_TABLE: &[Option<phf::Map<u32, usize>>] = &[];
    pub static EOF_TRANSITION_STATE: usize = usize::MAX;
    pub static EOF_TRANSITION_KIND: u32 = u32::MAX;
}

#[cfg(feature = "parser_generated")]
mod scan_rules {
    use crate::ScanPattern;
    include!("assets/generated/scan_rule.rs");
}
#[cfg(not(feature = "parser_generated"))]
mod scan_rules {
    use crate::ScanPattern;

    pub static LEXME_SCAN_RULE: phf::Map<char, &'static [ScanPattern]> = phf::phf_map!{};
    pub static REGEX_SCAN_RULE: &[ScanPattern] = &[];
    pub static SUPPORT_LEADING: &[usize] = &[];
    pub static SUPPORT_TRAILING: &[usize] = &[];
    pub static SUPPORT_MAIN: &[usize] = &[];
}

impl cstree::Syntax for SyntaxKind {
    fn from_raw(raw: cstree::RawSyntaxKind) -> Self {
        *kinds::SYNTAX_KIND_MAP.get(&raw.0).unwrap()
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

pub fn resolve_parser_next_state(state: usize, lookahead: &SyntaxKind) -> Result<LookaheadTransition, anyhow::Error> {
    let map = match states::LA_TRANSITION_TABLE.get(state) {
        Some(map) => map,
        None => {
            bail!("Invalid state on lookahead map (state#{state}).");
        }
    };
    
    match map.get(&lookahead.id) {
        Some(transition) => Ok(transition.clone()),
        None => {
            bail!("Invalid lookahead token on lookahead map (token: {:?}, state: {}).", lookahead, state);
        }
    }
}

pub fn resolve_parser_goto_state(state: usize, lhs_id: u32) -> Result<usize, anyhow::Error> {
    let map = match states::GOTO_TRANSITION_TABLE.get(state) {
        Some(Some(map)) => map,
        Some(None) => {
            bail!("No goto destination(s) (state#{state}).");
        }
        _ => {
            bail!("Invalid state on goto map (state#{state}).");
        }
    };
    
    match map.get(&lhs_id) {
        Some(next_state) => Ok(*next_state),
        None => {
            bail!("Invalid lhs symbol on goto map (symbol/id: {}, state#{})", lhs_id, state);
        }
    }
}

pub fn resolve_parser_accept_state(state: usize) -> Result<LookaheadTransition, anyhow::Error> {
    let last_state = states::EOF_TRANSITION_STATE;

    if state != last_state {
        return Err(anyhow::anyhow!("Invalid last state (current_state: {state}, expected state: {last_state})"));
    }

    Ok(LookaheadTransition::Accept { 
        last_state, 
        last_kind: SyntaxKind::from_raw(cstree::RawSyntaxKind(states::EOF_TRANSITION_KIND)) 
    })
}

pub fn eof_kind_id() -> u32 {
    states::EOF_TRANSITION_KIND
}

pub struct RegexScanPattern {
    pub kind: SyntaxKind,
    pub pattern: regex::Regex,
}

static REGEX_SCANS: std::sync::OnceLock<Vec<RegexScanPattern>> = std::sync::OnceLock::new();


pub fn regex_scan_patterns(support_scanners: &[usize]) -> Vec<&RegexScanPattern> {
    regex_scan_patterns_all().iter().enumerate()
    .filter(|(i, _)| support_scanners.contains(i))
    .map(|(_, pattern)| pattern)
    .collect()
}

fn regex_scan_patterns_all() -> &'static Vec<RegexScanPattern> {
    use cstree::Syntax;
    REGEX_SCANS.get_or_init(|| {
        scan_rules::REGEX_SCAN_RULE.iter()
        .map(|rule| RegexScanPattern {
            kind: SyntaxKind::from_raw(cstree::RawSyntaxKind(rule.id)),
            pattern: regex::Regex::new(rule.pattern).expect(&format!("Invalid regex scan pattern (patter: `{}`)", rule.pattern)),
        })
        .collect()
    })
}

pub fn support_leading() -> &'static [usize] {
    scan_rules::SUPPORT_LEADING
}

pub fn support_trailing() -> &'static [usize] {
    scan_rules::SUPPORT_TRAILING
}

pub fn support_main() -> &'static [usize] {
    scan_rules::SUPPORT_MAIN
}

pub fn scan_by_lexme_rule(source: &str) -> Option<crate::ScanPattern> {
    let (_, prefix) = source.char_indices().next().unwrap();

    let Some(patterns) = scan_rules::LEXME_SCAN_RULE.get(&prefix.to_ascii_lowercase()) else {
        return None;
    };

    match patterns.iter().filter(|p| source.starts_with(p.pattern)).next() {
        Some(pattern) => Some(pattern.clone()),
        None => None,
    }
}