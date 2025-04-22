mod scanner;

pub use scanner::{Scanner, ScannerScope};

use sqlite_parser_proto::SyntaxKind;

#[derive(Clone, Debug)]
pub struct Token {
    pub leading: Option<Vec<TokenItem>>,
    pub main: TokenItem,
    pub trailing: Option<Vec<TokenItem>>,
}

impl Token {
    pub fn offset_start(&self) -> usize {
        if let Some(leading) = self.leading.as_ref() {
            if let Some(offset) = leading.iter().map(|item| item.offset).next() {
                return offset;
            }
        }

        self.main.offset
    }

    pub fn token_len(&self) -> usize {
        let leading_len = self.leading.as_ref().map(|xs| xs.iter().map(|x| x.len).sum::<usize>()).unwrap_or_default();
        let trailing_len = self.trailing.as_ref().map(|xs| xs.iter().map(|x| x.len).sum::<usize>()).unwrap_or_default();

        leading_len + self.main.len + trailing_len
    }
}

#[derive(Clone, Debug)]
pub struct TokenItem {
    pub tag: SyntaxKind,
    pub offset: usize,
    pub len: usize,
    pub value: Option<String>,
}
