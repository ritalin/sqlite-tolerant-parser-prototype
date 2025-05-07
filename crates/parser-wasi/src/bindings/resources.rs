use std::cell::RefCell;

use super::parser_world::exports::ritalin::parser::parsers;
use super::parser_world::exports::ritalin::parser::syntax;

pub struct ParserImpl {
    inner: RefCell<::parser::Parser>,
}

impl parsers::GuestParser for ParserImpl {
    fn new() -> Self {
        Self { inner: RefCell::new(::parser::Parser::new()) }
    }

    fn parse(&self,source: String,) -> Result<parsers::SyntaxTree,parsers::ParserError> {
        match self.inner.borrow().parse(source) {
            Ok(tree) => Ok(parsers::SyntaxTree::new(SyntaxTreeImpl::new(tree))),
            Err(err) => Err(parsers::ParserError::SyntaxError(err.to_string()),)
        }
    }

    fn incremental(&self,tree: parsers::SyntaxTree,edit: parsers::EditScope,) -> Result<parsers::IncrementalParser,parsers::ParserError> {
        match self.inner.borrow().incremental(&tree.into(), edit.into()) {
            Ok(parser) => Ok(parsers::IncrementalParser::new(IncrementalParserImpl::new(parser))),
            Err(err) => Err(parsers::ParserError::IncrementalEditError(err.to_string())),
        }
    }
}

pub struct IncrementalParserImpl {
    inner: RefCell<::parser::IncrementalParser>,
}

impl IncrementalParserImpl {
    pub fn new(parser: ::parser::IncrementalParser) -> Self {
        Self { inner: RefCell::new(parser) }
    }
}

impl parsers::GuestIncrementalParser for IncrementalParserImpl {
    fn parse(&self,source: String,) -> Result<parsers::SyntaxTree,parsers::ParserError> {
        match self.inner.borrow().parse(source) {
            Ok(tree) => Ok(parsers::SyntaxTree::new(SyntaxTreeImpl::new(tree))),
            Err(err) => Err(parsers::ParserError::SyntaxError(err.to_string()),)
        }
    }
}

pub struct SyntaxTreeImpl {
    inner: ::parser::SyntaxTree,
}

impl SyntaxTreeImpl {
    pub fn new(tree: ::parser::SyntaxTree) -> Self {
        Self { inner: tree }
    }
}

impl syntax::GuestTree for SyntaxTreeImpl {
    fn root(&self,) -> syntax::Node {
        syntax::Node::new(SyntaxNodeImpl { inner: self.inner.root() })
    }
}

impl From<syntax::Tree> for ::parser::SyntaxTree {
    fn from(value: syntax::Tree) -> Self {
        value.get::<SyntaxTreeImpl>().inner.clone()
    }
}

pub struct SyntaxNodeImpl {
    inner: ::parser::SyntaxNode,
}

impl syntax::GuestNode for SyntaxNodeImpl {
    fn metadata(&self,) -> syntax::Metadata {
        let metadata = self.inner.metadata();

        syntax::Metadata {
            kind: scanner_wasi::scanner_types::SyntaxKind::from(&self.inner.kind()),
            node_type: metadata.node_type.into(),
            state: metadata.state as u64,
            recovery: metadata.recovery.map(From::from),
        }
    }

    fn offset_start(&self,) -> u32 {
        self.inner.text_range().start().into()
    }

    fn offset_end(&self,) -> u32 {
        self.inner.text_range().end().into()
    }

    fn value(&self,) -> Option<String> {
        self.inner.value()
    }
    
    fn leading_trivia(&self,) -> Vec::<syntax::Node> {
        self.inner.leading_trivia().iter().map(From::from).collect()
    }
    
    fn traling_trivia(&self,) -> Vec::<syntax::Node> {
        self.inner.trailing_trivia().iter().map(From::from).collect()
    }
    
    fn children(&self,) -> Vec::<syntax::Node> {
        self.inner.children().map(|node| From::from(&node)).collect()
    }
}

impl From<&::parser::SyntaxNode> for syntax::Node {
    fn from(value: &::parser::SyntaxNode) -> Self {
        syntax::Node::new(SyntaxNodeImpl { inner: value.clone() })
    }
}

impl From<::parser::NodeType> for syntax::NodeType {
    fn from(value: ::parser::NodeType) -> Self {
        match value {
            ::parser::NodeType::TokenSet => syntax::NodeType::TokenSet,
            ::parser::NodeType::LeadingToken => syntax::NodeType::LeadingToken,
            ::parser::NodeType::TrailingToken => syntax::NodeType::TrailingToken,
            ::parser::NodeType::MainToken => syntax::NodeType::MainToken,
            ::parser::NodeType::Node => syntax::NodeType::Node,
            ::parser::NodeType::Error => syntax::NodeType::Error,
            ::parser::NodeType::FatalError => syntax::NodeType::FatalError,
        }
    }
}

impl From<::parser::Recovery> for syntax::RecoveryStatus {
    fn from(value: ::parser::Recovery) -> Self {
        match value {
            ::parser::Recovery::Delete => syntax::RecoveryStatus::Delete,
            ::parser::Recovery::Shift => syntax::RecoveryStatus::Shift,
        }
    }
}

impl From<parsers::EditScope> for ::parser::EditScope {
    fn from(value: parsers::EditScope) -> Self {
        Self {
            offset: value.offset,
            from_len: value.from_len,
            to_len: value.to_len,
        }
    }
}

pub struct ParserComponent;

impl parsers::Guest for ParserComponent {
    type Parser = ParserImpl;
    type IncrementalParser = IncrementalParserImpl;
}

impl syntax::Guest for ParserComponent {
    type Tree = SyntaxTreeImpl;
    type Node = SyntaxNodeImpl;
}