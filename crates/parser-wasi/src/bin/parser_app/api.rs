mod bindings;

pub use bindings::ritalin::parser::parsers::Parser;

pub mod syntax {
    pub use super::bindings::ritalin::parser::syntax::*;
}