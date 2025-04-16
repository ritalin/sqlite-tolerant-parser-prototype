use std::collections::{LinkedList, VecDeque};

use anyhow::bail;
use cstree::{build::NodeCache, green::{GreenNode, GreenToken}, util::NodeOrToken, Syntax};
use parser::{InternCache, Language, Parser, SyntaxTree};
use sqlite_parser_proto::{LookaheadTransition, SyntaxKind, TransitionEvent};

#[cfg(feature = "parser_generated")]
use sqlite_parser_proto::engine::kinds as syntax_kind;


pub fn main() -> Result<(), anyhow::Error> {
    // #[cfg(not(feature = "parser_generated"))]
    // let mut tokens = VecDeque::<(SyntaxKind, Option<&str>)>::new();
    // #[cfg(feature = "parser_generated")]
    // let mut tokens = VecDeque::from_iter(
    //     vec![
    //         (syntax_kind::r#SELECT, None), 
    //         (syntax_kind::r#ID, Some("c")), (syntax_kind::r#DOT, Some(".")), (syntax_kind::r#ID, Some("code")), (syntax_kind::r#COMMA, Some(",")), 
    //         (syntax_kind::r#ID, Some("name")), 
    //         (syntax_kind::r#FROM, None), (syntax_kind::r#ID, Some("city")), (syntax_kind::r#ID, Some("c")), 
    //         (syntax_kind::r#SEMI, Some(";")), (syntax_kind::r#EOF, None)
    //     ].into_iter()
    // );

    let source = r#"
    /* 行頭Comment */
    SELECT * 
    FROM (
        /* なんたらかんたら */
        SELECT t.*, 'abc' || 'xyz' AS x, 123 / 456 AS y 
        FROM foo t 
        WHERE t.code = 10 -- 条件
    );
    "#;

    let parser = Parser::new();

    eprintln!("Process parsing....");

    let tree = parser.parse(&source)?;

    println!("{}", tree.debug(true));
    Ok(())
}
