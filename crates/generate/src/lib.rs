use std::collections::{BTreeMap, HashMap, LinkedList};

mod config;
mod convert;
mod export_syntax_kind;
mod export_state_transition;
mod export_scan_rule;

pub use config::ActionResolveConfig;
pub use convert::LalryBuilder;
pub use export_syntax_kind::{export_syntax_kind, export_syntax_kind_pretty};
pub use export_state_transition::{export_parser_state, export_parser_state_pretty};
pub use export_scan_rule::export_scan_rule_pretty;

struct IdGenerator {
    stack: LinkedList<u32>,
    next: u32,
}

impl IdGenerator {
    pub fn new(next_id: u32) -> Self {
        Self {
            stack: LinkedList::new(),
            next: next_id,
        }
    }

    pub fn set_current(&mut self, id: u32) {
        self.stack.push_back(id);
    }
    pub fn flush(&mut self) {
        self.stack.clear();
    }
    pub fn id(&mut self) -> u32 {
        match self.stack.pop_back() {
            Some(id) => id,
            None => {
                let id = self.next;
                self.next += 1;
                id
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct ScanRuleSet {
    pub lexme: HashMap<String, Vec<String>>,
    pub regex: BTreeMap<String, Vec<RegexScanRule>>,
}

#[derive(serde::Deserialize)]
pub struct RegexScanRule {
    pub pattern: String,
    #[serde(default)]
    pub leading: bool,
    #[serde(default)]
    pub trailing: bool,
    #[serde(default)]
    pub main: bool,
}

pub fn tokens_to_string(tokens: proc_macro2::TokenStream, depth: usize) -> String {
    let mut s = String::new();
    s.push_str(&"  ".repeat(depth));
    s.push_str(&tokens.to_string());
    s
}

pub fn with_indent(token_str: &str, depth: usize) -> String {
    let mut s = String::new();
    s.push_str(&"  ".repeat(depth));
    s.push_str(token_str);
    s
}
