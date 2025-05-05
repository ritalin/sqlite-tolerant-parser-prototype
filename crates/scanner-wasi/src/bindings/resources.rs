use std::cell::RefCell;
use cstree::RawSyntaxKind;
use ::scanner::{Scanner};
use sqlite_parser_proto::SyntaxKind;
use super::scanner_world::exports::ritalin::scanner::{scanner, types};

pub struct ScannerImpl {
    inner: RefCell<Scanner>
}

impl ScannerImpl {
    fn new(source: String,index_from: u64) -> ScannerImpl {
        Self { inner: RefCell::new(Scanner::create(source, index_from as usize)) }
    }
}

impl scanner::GuestScanner for ScannerImpl {
    fn create(source: String,index_from: u64,) -> scanner::Scanner {
        scanner::Scanner::new(Self::new(source, index_from))
    }

    fn lookahead(&self,) -> Option<scanner::Token> {
        self.inner.borrow().lookahead().map(|x| From::from(x.clone()))
    }

    fn shift(&self,) -> Option<scanner::Token> {
        self.inner.borrow_mut().shift().map(|x| From::from(x.clone()))
    }

    fn scope(&self,) -> scanner::ScannerScope {
        self.inner.borrow().scope().into()
    }

    fn revert(&self,scope: scanner::ScannerScope,) -> () {
        self.inner.borrow_mut().revert(scope.into());
    }
}

impl From<::scanner::Token> for types::Token {
    fn from(value: ::scanner::Token) -> Self {
        Self { 
            leading: value.leading.map(|trivia| trivia.iter().map(Into::into).collect()), 
            main: From::from(&value.main), 
            trailing: value.trailing.map(|trivia| trivia.iter().map(Into::into).collect())
        }
    }
}
impl From<types::Token> for ::scanner::Token {
    fn from(value: types::Token) -> Self {
        Self { 
            leading: value.leading.map(|trivia| trivia.iter().map(Into::into).collect()), 
            main: From::from(&value.main), 
            trailing: value.trailing.map(|trivia| trivia.iter().map(Into::into).collect())
        }
    }
}

impl From<&types::TokenItem> for ::scanner::TokenItem {
    fn from(value: &types::TokenItem) -> Self {
        Self { 
            tag: From::from(&value.kind), 
            offset: value.offset as usize, 
            len: value.len as usize, 
            value: value.value.clone(),
        }
    }
}
impl From<&::scanner::TokenItem> for types::TokenItem {
    fn from(value: &::scanner::TokenItem) -> Self {
        Self { 
            kind: From::from(&value.tag), 
            offset: value.offset as u64, 
            len: value.len as u64, 
            value: value.value.clone(),
        }
    }
}

impl From<&sqlite_parser_proto::SyntaxKind> for types::SyntaxKind {
    fn from(value: &sqlite_parser_proto::SyntaxKind) -> Self {
        Self {
            id: value.id,
            text: value.text.into(),
            is_keyword: value.is_keyword,
            is_terminal: value.is_terminal,
        }
    }
}
impl From<&types::SyntaxKind> for sqlite_parser_proto::SyntaxKind {
    fn from(value: &types::SyntaxKind) -> Self {
        use cstree::Syntax;
        SyntaxKind::from_raw(RawSyntaxKind(value.id))
    }
}


impl From<::scanner::ScannerScope> for types::ScannerScope {
    fn from(value: ::scanner::ScannerScope) -> Self {
        Self { next_index: value.next_index as u64, lookahead: value.lookahead.map(Into::into) }
    }
}
impl From<types::ScannerScope> for ::scanner::ScannerScope {
    fn from(value: scanner::ScannerScope) -> Self {
        Self { next_index: value.next_index as usize, lookahead: value.lookahead.map(Into::into) }    
    }
}

pub struct ScannerComponent;
impl<'a> scanner::Guest for ScannerComponent {
    type Scanner = ScannerImpl;
}
