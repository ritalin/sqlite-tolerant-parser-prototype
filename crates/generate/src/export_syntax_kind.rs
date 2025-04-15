use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use sqlite_parser_proto::{GrammarSymbol, SymbolType};

use crate::tokens_to_string;

pub fn export_syntax_kind(symbols: &[GrammarSymbol]) -> String {
    // use super::SyntaxKind;
    // #[allow(non_upper_case_globals)] pub static r#SELECT: SyntaxKind = SyntaxKind { text: "SELECT", id: 201, is_keyword: true, is_terminal: true };

    let kinds: TokenStream = symbols.iter()
        .map(|symbol| create_syntax_kind_token(symbol))
        .collect()
    ;

    let kind_map_members: TokenStream = symbols.iter()
        .map(|symbol| create_syntax_kind_map_member(symbol))
        .collect()
    ;

    quote! {
        use phf::phf_map;
        #kinds
        pub static SYNTAX_KIND_MAP: phf::Map<u32, SyntaxKind> = phf::phf_map!{#kind_map_members};
    }
    .to_string()
}

fn create_syntax_kind_token(symbol: &GrammarSymbol) -> TokenStream {
    let ident = format_ident!("r#{}", symbol.name);
    let text = symbol.name.clone();
    let id = symbol.id;
    let is_keyword = if let SymbolType::Terminal { is_keyword } = symbol.symbol_type { is_keyword } else { false };
    let is_terminal = if let SymbolType::Terminal { .. } = symbol.symbol_type { true } else { false };

    quote! {
        #[allow(non_upper_case_globals)] pub static #ident: SyntaxKind = SyntaxKind { text: #text, id: #id, is_keyword: #is_keyword, is_terminal: #is_terminal };
    }
}

fn create_syntax_kind_map_member(symbol: &GrammarSymbol) -> TokenStream {
    let ident = format_ident!("r#{}", symbol.name);
    let id = symbol.id;

    quote! {
        #id => #ident,
    }
}

pub fn export_syntax_kind_pretty(symbols: &[GrammarSymbol]) -> String {
    let kinds = symbols.iter()
        .map(|symbol| create_syntax_kind_token(symbol))
        .map(|token| tokens_to_string(token, 0))
    ;

    let kind_map_members = symbols.iter()
        .map(|symbol| create_syntax_kind_map_member(symbol))
        .map(|token| tokens_to_string(token, 1))
    ;

    let iter = std::iter::empty();
    iter.chain(vec!["use phf::phf_map;".to_string()])
        .chain(kinds)
        .chain(vec!["pub static SYNTAX_KIND_MAP: phf::Map<u32, SyntaxKind> = phf_map!{".to_string()])
        .chain(kind_map_members)
        .chain(vec!["};".to_string()])
        .collect::<Vec<_>>()
        .join("\n")
}