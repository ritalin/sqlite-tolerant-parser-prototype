use parser::{Parser, SyntaxTree};

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

    let tree = parser.parse(source.into())?;

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
    dump_tree_internal(tree.root(), 0);
}

fn dump_tree_internal(node: ::parser::SyntaxNode, indent: usize) {
    let range = node.text_range();
    let kind = node.kind();
    let range_str = format!("{} - {}", usize::from(range.start()), usize::from(range.end()));
    let indent_str = std::iter::repeat("  ").take(indent).collect::<String>();
    let value = node.value().map(|v| v.replace("\n", r"\n")).unwrap_or_default();
    let node_type = node.metadata().node_type;

    println!("{:<16}{:<24}{}{} ({}) {}", 
        range_str, format!("{:?}", node_type),
        indent_str, kind.text, kind.id, value
    );

    for child in node.children() {
        dump_tree_internal(child, indent + 1);
    }
}

#[cfg(test)]
mod parser_tests {
    use cstree::text::{TextRange, TextSize};
    use parser::{NodeType, Recovery};
    use sqlite_parser_proto::engine::kinds as syntax_kind;
    use super::*;

    #[test]
    fn test_multiple_statement() -> Result<(), anyhow::Error> {
        let source = "SELECT 123 FROM foo;SELECT 42;";
        let parser = Parser::new();
        let tree = parser.parse(source.into())?;

        // dump_tree(&tree);

        assert_eq!(3, tree.root().children().count());

        let kinds = tree.root().children().enumerate().map(|(i, node)| (i, node.kind().text)).collect::<Vec<_>>();
        assert_eq!(vec![(0,"ecmd" ), (1,"ecmd" ), (2,"ecmd" ), ], kinds);
        Ok(())
    }

    #[test]
    fn test_unmatch_token_query() -> Result<(), anyhow::Error> {
        let source = "SELECT 123 DELETE FROM foo;";
        let parser = Parser::new();
        let tree = parser.parse(source.into())?;

        dump_tree(&tree);

        let element = tree.covering_element(TextRange::new(TextSize::new(11), TextSize::new(17)));
        let Some(node) = element else {
            panic!("Covering element does not exist.");
        };
        
        'error_node: {
            assert_eq!(syntax_kind::r#DELETE, node.kind());

            let annotation = node.metadata();
            assert_eq!(NodeType::Error, annotation.node_type);
            assert_eq!(Some(Recovery::Delete), annotation.recovery);
            break 'error_node;
        }
        
        Ok(())
    }

    #[test]
    fn test_brank_token_query() -> Result<(), anyhow::Error> {
        let source = "SELECT  FROM foo;";
        let parser = Parser::new();
        let tree = parser.parse(source.into())?;

        dump_tree(&tree);

        let element = tree.covering_element(TextRange::new(TextSize::new(8), TextSize::new(8)));
        let Some(error_node) = element else {
            panic!("Covering element does not exist.");
        };
        
        'error_node: {
            assert_eq!(syntax_kind::r#ILLEGAL, error_node.kind());

            let annotation = error_node.metadata();
            assert_eq!(NodeType::Error, annotation.node_type);
            assert_eq!(Some(Recovery::Shift), annotation.recovery);
            break 'error_node;
        }
        
        Ok(())
    }

    #[test]
    fn test_incremental_parse_repairing() -> Result<(), anyhow::Error> {
        let source0 = "SELECT  FROM foo;";
        let parser = Parser::new();
        let tree0 = parser.parse(source0.into())?;

        // dump_tree(&tree0);
        // eprintln!("[DEBUG] pre-orders: {:?}", 
        //     tree0.root().preorder_with_tokens()
        //     .filter_map(|event| match event {
        //         cstree::traversal::WalkEvent::Enter(node) => Some(node),
        //         cstree::traversal::WalkEvent::Leave(_) => None,
        //     })
        //     .map(|node| (node.kind().text, node.text_range())).collect::<Vec<_>>()
        // );

        let source = "SELECT 123 FROM foo;";
        let inc_parser = parser.incremental(&tree0, 
            parser::EditScope { offset: 7, from_len: 0, to_len: 3 }
        )?;
        let tree = inc_parser.parse(source.to_string())?;

        // eprintln!(">>> Dump AnnotationKeys");
        // tree.root().preorder_with_tokens().filter_map(|event| match event {
        //     cstree::traversal::WalkEvent::Enter(NodeOrToken::Node(node)) => {
        //         Some(("Node", AnnotationKey::from(node.syntax())))
        //     }
        //     cstree::traversal::WalkEvent::Enter(NodeOrToken::Token(node)) => {
        //         Some(("Token", AnnotationKey::from(node.syntax())))
        //     }
        //     cstree::traversal::WalkEvent::Leave(_) => None,
        // })
        // .for_each(|(tag, key)| {
        //     eprintln!("key: {:?}, tag: {}", key, tag);
        // });
        // eprintln!("<<<\n----------");
        // eprintln!(">>> Dump Annotations");
        // tree.annotations.iter()
        // .map(|(key, (id, a))| (key, a.node_type.clone(), id))
        // .for_each(|(key, node_type, id)| {
        //     eprintln!("key: {:?}, type: {:?}, id: {:?}", key, node_type, id);
        // });
        // eprintln!("<<<\n----------");

        dump_tree(&tree);

        Ok(())
    }

    #[test]
    fn test_incremental_parse_dropping() -> Result<(), anyhow::Error> {
        let source0 = "SELECT * FROM foo;";
        let parser = Parser::new();
        let tree0 = parser.parse(source0.into())?;

        // dump_tree(&tree0);

        let source = "SELECT  FROM foo;";
        let inc_parser = parser.incremental(&tree0, 
            parser::EditScope { offset: 7, from_len: 1, to_len: 0 }
        )?;
        let tree = inc_parser.parse(source.to_string())?;

        dump_tree(&tree);

        Ok(())
    }

    #[test]
    fn test_incremental_parse_breaking() -> Result<(), anyhow::Error> {
        let source0 = "SELECT 123 FROM foo;";
        let parser = Parser::new();
        let tree0 = parser.parse(source0.into())?;

        // dump_tree(&tree0);

        let source = "SELECT 123 FROMbar foo;";
        let inc_parser = parser.incremental(&tree0, 
            parser::EditScope { offset: 11, from_len: 4, to_len: 7 }, 
        )?;
        let tree = inc_parser.parse(source.to_string())?;

        dump_tree(&tree);
        Ok(())
    }

    #[test]
    fn test_incremental_parse_fatal() -> Result<(), anyhow::Error> {
        let source0 = "SELECT 123 123 123 123 FROM foo a;";
        let parser = Parser::new();
        let tree = parser.parse(source0.into())?;

        dump_tree(&tree);
        Ok(())
    }

    #[test]
    fn test_incremental_parse_braking_fatal() -> Result<(), anyhow::Error> {
        let source0 = "SELECT 123 FROM foo;";
        let parser = Parser::new();
        let tree0 = parser.parse(source0.into())?;

        let source = "SELECT 123 123 123 123 FROM foo a;";
        let parser = Parser::new();
        let inc_parser = parser.incremental(&tree0, 
            parser::EditScope { offset: 11, from_len: 0, to_len: 12 }, 
        )?;
        let tree = inc_parser.parse(source.to_string())?;

        dump_tree(&tree);
        Ok(())
    }
}