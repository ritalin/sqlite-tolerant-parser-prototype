

use cstree::{syntax::{SyntaxNode, SyntaxToken}, util::NodeOrToken};
use parser::{Parser, SyntaxTree};
use sqlite_parser_proto::{engine::kinds as syntax_kind, SyntaxKind};


pub fn main() -> Result<(), anyhow::Error> {
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

    let tree = parser.parse(&source)?;

    // println!("{}", tree.debug(true));
    dump_tree(&tree);
    Ok(())
}

fn dump_tree(tree: &SyntaxTree) {
    dump_tree_internal(NodeOrToken::Node(tree.root()), 0);
}

fn dump_tree_internal(node: NodeOrToken<&SyntaxNode<SyntaxKind>, &SyntaxToken<SyntaxKind>>, indent: usize) {
    let range = node.text_range();
    let kind = node.kind();
    let range_str = format!("{} - {}", usize::from(range.start()), usize::from(range.end()));
    let indent_str = std::iter::repeat("    ").take(indent).collect::<String>();

    println!("{:<16}{}{} ({})", 
        range_str, indent_str, kind.text, kind.id,
    );

    if let NodeOrToken::Node(node) = node {
        for child in node.children_with_tokens() {
            dump_tree_internal(child, indent + 1);
        }
    }
}