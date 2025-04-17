mod scanner;

pub use scanner::Scanner;

use sqlite_parser_proto::SyntaxKind;

#[derive(Clone, Debug)]
pub struct Token {
    pub leading: Option<Vec<TokenItem>>,
    pub main: TokenItem,
    pub trailing: Option<Vec<TokenItem>>,
}

#[derive(Clone, Debug)]
pub struct TokenItem {
    pub tag: SyntaxKind,
    pub offset: usize,
    pub len: usize,
    pub value: Option<String>,
}
