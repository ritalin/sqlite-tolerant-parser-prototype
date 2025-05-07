mod api;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = "SELECT 123 123 123 FROM foo a;";
    let parser = api::Parser::new();
    let tree = parser.parse(source)?;

    dump_tree(&tree);

    Ok(())
}

fn dump_tree(tree: &api::syntax::Tree) {
    dump_tree_internal(tree.root(), 0);
}

fn dump_tree_internal(node: api::syntax::Node, indent: usize) {
    let metadata = node.metadata();
    let range_str = format!("{} - {}", node.offset_start(), node.offset_end());
    let indent_str = std::iter::repeat("  ").take(indent).collect::<String>();
    let value = node.value().map(|v| format!("`{}`", v.replace("\n", r"\n"))).unwrap_or_default();
    let node_type = node.metadata().node_type;

    println!("{:<16}{:<24}{}{} ({}) {}", 
        range_str, format!("{:?}", node_type),
        indent_str, metadata.kind.text, metadata.kind.id, value
    );

    for child in node.children() {
        dump_tree_internal(child, indent + 1);
    }
}