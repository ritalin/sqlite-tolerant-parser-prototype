

use std::collections::HashMap;

use cstree::{syntax::{SyntaxNode, SyntaxToken}, util::NodeOrToken};
use parser::{AnnotationKey, Parser, SyntaxTree};
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

    // let mut annotations = Vec::from_iter(tree.annotations.iter());
    // annotations.sort_by(|(lkey, _), (rkey, _)| {
    //     let lhs = lkey.offset..lkey.offset+lkey.len;
    //     let rhs = rkey.offset..rkey.offset+rkey.len;
    //     lhs.cmp(rhs)
    // });

    // for (k, v) in annotations {
    //     println!("{:<4?} {:?}", k, v);
    // }

    Ok(())
}

fn dump_tree(tree: &SyntaxTree) {
    dump_tree_internal(NodeOrToken::Node(tree.root()), &tree.annotations, 0);
}

fn dump_tree_internal(node: NodeOrToken<&SyntaxNode<SyntaxKind>, &SyntaxToken<SyntaxKind>>, annotations: &HashMap<AnnotationKey, (parser::NodeId, parser::Annotation)>, indent: usize) {
    let range = node.text_range();
    let kind = node.kind();
    let range_str = format!("{} - {}", usize::from(range.start()), usize::from(range.end()));
    let indent_str = std::iter::repeat("  ").take(indent).collect::<String>();

    let key = match node {
        NodeOrToken::Node(x) => AnnotationKey::from(x),
        NodeOrToken::Token(x) => AnnotationKey::from(x),
    };

    let value = match node {
        NodeOrToken::Node(_) => "".to_string(),
        NodeOrToken::Token(x) => format!("`{}`", x.resolved().text().replace("\n", r"\n")),
    };

    let node_type = annotations.get(&key).map(|(_, a)| format!("{:?}", a.node_type)).unwrap_or("?".to_string());

    println!("{:<16}{:<16}{}{} ({}) {}", 
        range_str, node_type,
        indent_str, kind.text, kind.id, value
    );

    if let NodeOrToken::Node(node) = node {
        for child in node.children_with_tokens() {
            dump_tree_internal(child, annotations, indent + 1);
        }
    }
}

#[cfg(test)]
mod parser_tests {
    use cstree::text::{TextRange, TextSize};
    use parser::{NodeType, Recovery};
    use super::*;

    #[test]
    fn test_unmatch_token_query() -> Result<(), anyhow::Error> {
        let source = "SELECT 123 123 FROM foo;";
        let parser = Parser::new();
        let tree = parser.parse(&source)?;

        // dump_tree(&tree);

        let element = tree.root().covering_element(TextRange::new(TextSize::new(11), TextSize::new(14)));
        let Some(node) = element.as_token() else {
            panic!("Covering element does not exist.");
        };
        
        'error_node: {
            assert_eq!(syntax_kind::r#INTEGER, node.parent().kind());

            let error_node = node.parent();
            let Some(annotation) = tree.get_annotation_of(AnnotationKey::from(error_node.syntax())) else {
                panic!("Node annotation for parent must be assigned.");
            };
            assert_eq!(NodeType::Error, annotation.node_type);
            assert_eq!(Some(Recovery::Delete), annotation.recovery);
            break 'error_node;
        }
        
        Ok(())
    }

    #[test]
    fn test_brank_token_query() -> Result<(), anyhow::Error> {
        // 'token_set: {
        //     assert_eq!(syntax_kind::r#DOT, node.kind());

        //     let Some(annotation) = tree.get_annotation_of(cstree::util::NodeOrToken::Node(node)) else {
        //         panic!("Node annotation must be assigned.");
        //     };
        //     assert_eq!(NodeType::TokenSet, annotation.node_type);

        //     let children = node.children().collect::<Vec<_>>();
        //     assert_eq!(2, children.len());
        //     'token: {
        //         assert_eq!(syntax_kind::r#DOT, children[0].kind());
        //         let Some(annotation) = tree.get_annotation_of(cstree::util::NodeOrToken::Node(children[0])) else {
        //             panic!("Node annotation must be assigned.");
        //         };
        //         assert_eq!(NodeType::MainToken, annotation.node_type);
        //         break 'token;
        //     }
        //     'token: {
        //         assert_eq!(syntax_kind::r#SPACE, children[0].kind());
        //         let Some(annotation) = tree.get_annotation_of(cstree::util::NodeOrToken::Node(children[0])) else {
        //             panic!("Node annotation must be assigned.");
        //         };
        //         assert_eq!(NodeType::TrailingToken, annotation.node_type);
        //         break 'token;
        //     }
        //     break 'token_set;
        // };
       todo!()
    }
}