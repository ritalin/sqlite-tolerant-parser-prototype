use std::collections::{BTreeMap, HashMap};
use proc_macro2::TokenStream;
use sqlite_parser_proto::GrammarSymbol;
use quote::quote;

use crate::{tokens_to_string, with_indent, ScanRuleSet};

pub fn export_scan_rule_pretty(rule_set: &ScanRuleSet, symbols: &[GrammarSymbol], lookup: &HashMap<String, u32>) -> String {
    std::iter::empty()
        .chain(vec![
            "use phf::phf_map;".to_string(),
        ])
        .chain(exprt_lexme_scan_rule_pretty(&rule_set.lexme, collect_keywords(symbols), lookup))
        .chain(export_regex_scan_rule_pretty(&rule_set.regex, lookup))
        .chain(export_alternative_token_pretty(&rule_set.alternatives, lookup))
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_keywords(symbols: &[GrammarSymbol]) -> impl Iterator<Item = GrammarSymbol> {
    symbols.iter().filter_map(|symbol| match symbol.symbol_type {
        sqlite_parser_proto::SymbolType::Terminal { is_keyword } if is_keyword => {
            Some(symbol.clone())
        }
        _ => None,
    })
}

fn export_rule_pattern(id: u32, name: &str, len: usize) -> TokenStream {
    quote! { ScanPattern { id: #id, pattern: #name, len: #len }, }
}

fn exprt_lexme_scan_rule_pretty(lexme: &HashMap<String, Vec<String>>, keywords: impl Iterator<Item = GrammarSymbol>, lookup: &HashMap<String, u32>) -> impl Iterator<Item = String> {
    let mut scan_rules = BTreeMap::<char, Vec<String>>::new();
    
    for symbol in keywords {
        let rule = tokens_to_string(export_rule_pattern(symbol.id, &symbol.name, symbol.name.len()), 2);
        let (_, prefix) = symbol.name.char_indices().next().unwrap();

        scan_rules.entry(prefix.to_ascii_lowercase())
            .and_modify(|xs| xs.push(rule.clone()))
            .or_insert_with(|| vec![rule])
        ;
    }

    for (name, patterns) in lexme {
        let id = *lookup.get(name).unwrap();

        for pattern in patterns {
            let rule = tokens_to_string(export_rule_pattern(id, &pattern, pattern.len()), 2);
            let (_, prefix) = pattern.char_indices().next().unwrap();

            scan_rules.entry(prefix.to_ascii_lowercase())
                .and_modify(|xs| xs.push(rule.clone()))
                .or_insert_with(|| vec![rule])
            ;
        }
    }

    let scan_rules = scan_rules.into_iter()
        .flat_map(|(prefix, rules)| {
            std::iter::empty()
            .chain(vec![
                with_indent(&format!("'{prefix}' => &["), 1)
            ])
            .chain(rules)
            .chain(vec![
                with_indent("],", 1)
            ])
        })
        .collect::<Vec<_>>()
    ;
    
    std::iter::empty()
    .chain(
        vec!["pub static LEXME_SCAN_RULE: phf::Map<char, &'static [ScanPattern]> = phf_map!{".to_string()]
    )
    .chain(scan_rules)
    .chain(
        vec!["};".to_string()]
    )
}

fn export_regex_scan_rule_pretty(regex: &BTreeMap<String, Vec<crate::RegexScanRule>>, lookup: &HashMap<String, u32>) -> impl Iterator<Item = String> {
    let mut scan_rules = Vec::<String>::new();
    let mut support_leading = vec![];
    let mut support_trailing = vec![];
    let mut support_main = vec![];

    let mut i: usize = 0;

    for (name, patterns) in regex {
        let id = *lookup.get(name).unwrap();

        for pattern in patterns {
            let rule = tokens_to_string(export_rule_pattern(id, &pattern.pattern, pattern.pattern.len()), 1);

            scan_rules.push(rule.clone());

            let support_index = with_indent(&export_rule_support_pretty(i, &pattern.pattern), 1);
            if pattern.leading { 
                support_leading.push(support_index.clone()); 
            }
            if pattern.trailing { 
                support_trailing.push(support_index.clone()); 
            }
            if pattern.main { 
                support_main.push(support_index.clone()); 
            }
            i += 1;
        }
    }

    std::iter::empty()
    .chain(vec![
        "pub static REGEX_SCAN_RULE: &[ScanPattern] = &[".to_string()
    ])
    .chain(scan_rules)
    .chain(vec![
        "];".to_string(),
        "pub static SUPPORT_LEADING: &[usize] = &[".to_string()
    ])
    .chain(support_leading)
    .chain(vec![
        "];".to_string(),
        "pub static SUPPORT_TRAILING: &[usize] = &[".to_string()
    ])
    .chain(support_trailing)
    .chain(vec![
        "];".to_string(),
        "pub static SUPPORT_MAIN: &[usize] = &[".to_string()
    ])
    .chain(support_main)
    .chain(vec![
        "];".to_string()
    ])
}

fn export_alternative_token_pretty(alternatives: &HashMap<String, Vec<String>>, lookup: &HashMap<String, u32>) -> impl Iterator<Item = String> {
    let rules = alternatives.iter()
        .flat_map(|(symbol, values)| export_alternative_pattern_pretty(symbol, values, lookup))
    ;

    std::iter::empty()
    .chain(vec!["pub static ALTERNATIVE_SYMBOL_TABLE: phf::Map<u32, &[u32]> = phf_map!{".to_string()])
    .chain(rules)
    .chain(vec!["};".to_string()])
}

fn export_alternative_pattern_pretty(symbol: &str, alternatives: &[String], lookup: &HashMap<String, u32>) -> impl Iterator<Item = String> {
    let key = lookup.get(symbol).expect(&format!("Not found alternative key (`{symbol}`)"));

    let values = alternatives.iter()
        .map(|alt| {
            let id = lookup.get(alt).expect(&format!("Not found alternative value (`{alt}`)"));
            with_indent(&export_rule_support_pretty(*id, &alt), 2)
        })
    ;

    std::iter::empty()
    .chain(vec![with_indent(&format!("{key}u32 => &["), 1)])
    .chain(values)
    .chain(vec![with_indent("],", 1)])
}

fn export_rule_support_pretty<V: std::fmt::Display>(i: V, pattern: &str) -> String {
    format!("{i}, // {pattern}")
}
