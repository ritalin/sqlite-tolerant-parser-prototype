import {parsers, syntax} from 'pkg/parser/parser'

export function setupCounter(_element: HTMLButtonElement) {
  const source = "SELECT 123 123 123 FROM foo a;";

  console.log("query: `SELECT 123 123 123 FROM foo a;`")
  console.log("------------------------------------------------------------")

  const parser = new parsers.Parser()
  const tree = parser.parse(source)

  dump_tree(tree)
}

function dump_tree(tree: syntax.Tree) {
  dump_tree_internal(tree.root(), 0);
}

function dump_tree_internal(node: syntax.Node, indent: number) {
  let metadata = node.metadata();
  let rangeStr = `${node.offsetStart()} - ${node.offsetEnd()}`;
  let indentStr = padLeft(' ', indent * 2);
  let value = node.value() ? `"${node.value()?.replace(/\n/g, "\\n")}"` : "";
  let nodeType = node.metadata().nodeType;

  console.log(`${padLeft(rangeStr, 16)}${padLeft(nodeType, 24)}${indentStr}${metadata.kind.text} (${metadata.kind.id}) ${value}`);

  for (const child of node.children()) {
      dump_tree_internal(child, indent + 1);
  }
}

function padLeft(str: string, totalWidth: number, padChar = ' '): string {
  return str.padStart(totalWidth, padChar);
}