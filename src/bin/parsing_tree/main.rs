use std::collections::LinkedList;

use cstree::{green::{GreenNode, GreenToken}, syntax::SyntaxNode, util::NodeOrToken, Syntax};



pub fn main() {
    use syntax_kind;

  let statuses = vec![
    ParseStatus::Shift { kind: syntax_kind::SELECT, value: None },
    ParseStatus::Reduce{ kind: syntax_kind::distinct, len: 0 }, 
    ParseStatus::Reduce{ kind: syntax_kind::sclp, len: 0 },
    ParseStatus::Reduce{ kind: syntax_kind::scanpt, len: 0 },
    ParseStatus::Shift{  kind: syntax_kind::ID, value: Some("c".into()) },
    ParseStatus::Reduce{ kind: syntax_kind::nm, len: 1 }, 
    ParseStatus::Shift{  kind: syntax_kind::DOT, value: Some(".".into()) },
    ParseStatus::Shift{  kind: syntax_kind::ID, value: Some("code".into()) }, 
    ParseStatus::Reduce{ kind: syntax_kind::nm, len: 1 },
    ParseStatus::Reduce{ kind: syntax_kind::expr, len: 3 },
    ParseStatus::Reduce{ kind: syntax_kind::scanpt, len: 0 },
    ParseStatus::Reduce{ kind: syntax_kind::r#as, len: 0 },
    ParseStatus::Reduce{ kind: syntax_kind::selcollist, len: 5 }, 
    ParseStatus::Shift{  kind: syntax_kind::COMMA, value: Some(",".into()) },
    ParseStatus::Reduce{ kind: syntax_kind::sclp, len: 2 },
    ParseStatus::Reduce{ kind: syntax_kind::scanpt, len: 0 }, 
    ParseStatus::Shift{  kind: syntax_kind::ID, value: Some("name".into()) },
    ParseStatus::Reduce{ kind: syntax_kind::expr, len: 1 },
    ParseStatus::Reduce{ kind: syntax_kind::scanpt, len: 0 },
    ParseStatus::Reduce{ kind: syntax_kind::r#as, len: 0 },
    ParseStatus::Reduce{ kind: syntax_kind::selcollist, len: 5 },
    ParseStatus::Shift{  kind: syntax_kind::FROM, value: None }, 
    ParseStatus::Reduce{ kind: syntax_kind::stl_prefix, len: 0 }, 
    ParseStatus::Shift{  kind: syntax_kind::ID, value: Some("city".into()) },
    ParseStatus::Reduce{ kind: syntax_kind::nm, len: 1 }, 
    ParseStatus::Reduce{ kind: syntax_kind::dbnm, len: 0 }, 
    ParseStatus::Shift{  kind: syntax_kind::ID, value: Some("c".into()) }, 
    ParseStatus::Reduce{ kind: syntax_kind::r#as, len: 1 }, 
    ParseStatus::Reduce{ kind: syntax_kind::on_using, len: 0 }, 
    ParseStatus::Reduce{ kind: syntax_kind::seltablist, len: 5 }, 
    ParseStatus::Reduce{ kind: syntax_kind::from, len: 2 }, 
    ParseStatus::Reduce{ kind: syntax_kind::where_opt, len: 0 },  
    ParseStatus::Reduce{ kind: syntax_kind::groupby_opt, len: 0 },
    ParseStatus::Reduce{ kind: syntax_kind::having_opt, len: 0 }, 
    ParseStatus::Reduce{ kind: syntax_kind::orderby_opt, len: 0 },
    ParseStatus::Reduce{ kind: syntax_kind::limit_opt, len: 0 },  
    ParseStatus::Reduce{ kind: syntax_kind::oneselect, len: 9 },  
    ParseStatus::Reduce{ kind: syntax_kind::selectnowith, len: 1 }, 
    ParseStatus::Reduce{ kind: syntax_kind::select, len: 1 },  
    ParseStatus::Reduce{ kind: syntax_kind::cmd, len: 1 },  
    ParseStatus::Reduce{ kind: syntax_kind::cmdx, len: 1 }, 
    ParseStatus::Shift{  kind: syntax_kind::SEMI, value: Some(";".into()) },
    ParseStatus::Reduce{ kind: syntax_kind::ecmd, len: 2 }, 
    ParseStatus::Reduce{ kind: syntax_kind::cmdlist, len: 1 }, 
    ParseStatus::Reduce{ kind: syntax_kind::input, len: 1 }, 
    ParseStatus::Shift{  kind: syntax_kind::EOF, value: None },
    ParseStatus::Accept{ kind: syntax_kind::program },
  ];

    
    let mut node_stack = LinkedList::<Option<NodeOrToken::<GreenNode, GreenToken>>>::new();
    let mut cache = cstree::build::NodeCache::new();

    for status in &statuses {
        let node = match status {
            ParseStatus::Shift { kind, value } => {
                let mut builder = cstree::build::GreenNodeBuilder::with_cache(&mut cache);
                builder.start_node(*kind);

                match (kind.is_keyword, kind.is_terminal) {
                    (true, true) => {
                        builder.static_token(*kind);
                    }
                    (false, true) if value.is_some() => {
                        let s = value.as_ref().unwrap();
                        builder.token(*kind, s);
                    }
                    _ => continue
                }

                builder.finish_node();
                let node = builder.finish().0.children().next()
                    .and_then(|x| x.into_token())
                    .map(|x| NodeOrToken::<GreenNode, GreenToken>::Token(x.clone()))
                ;
                node
            }
            ParseStatus::Reduce { kind, len } if *len > 0 => {
                let children = node_stack.split_off(node_stack.len() - len)
                    .into_iter()
                    .filter_map(std::convert::identity)
                    .map(Into::into)
                    .collect::<Vec<_>>()
                ;
                let node = GreenNode::new(kind.into_raw(), children);

                Some(NodeOrToken::<GreenNode, GreenToken>::Node(node))
            }
            ParseStatus::Reduce { .. } => {
                None
            }
            ParseStatus::Accept { kind } => {
                let children = node_stack.clone()
                    .into_iter()
                    .filter_map(std::convert::identity)
                    .map(Into::into)
                    .collect::<Vec<_>>()
                ;
                let node = GreenNode::new(kind.into_raw(), children);

                Some(NodeOrToken::<GreenNode, GreenToken>::Node(node))
            }
        };

        node_stack.push_back(node);
    }

    if let Some(root) = node_stack.pop_back().flatten().and_then(|x| x.into_node()) {
        let red_node = SyntaxNode::<SyntaxKind>::new_root(From::from(root));
        let s = red_node.debug(cache.interner(), true);
        println!("{s}");
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
pub struct SyntaxKind {
    id: u32,
    text: &'static str,
    is_keyword: bool,
    is_terminal: bool,
}

#[allow(non_camel_case_types)]
pub mod syntax_kind {
    use super::SyntaxKind;

    #[allow(non_upper_case_globals)] pub static r#SELECT: SyntaxKind = SyntaxKind { text: "SELECT", id: 1, is_keyword: true, is_terminal: true };
    #[allow(non_upper_case_globals)] pub static r#ID: SyntaxKind = SyntaxKind { text: "ID", id: 2, is_keyword: false, is_terminal: true };
    #[allow(non_upper_case_globals)] pub static r#DOT: SyntaxKind = SyntaxKind { text: "DOT", id: 3, is_keyword: true, is_terminal: true };
    #[allow(non_upper_case_globals)] pub static r#COMMA: SyntaxKind = SyntaxKind { text: "COMMA", id: 4, is_keyword: true, is_terminal: true };
    #[allow(non_upper_case_globals)] pub static r#FROM: SyntaxKind = SyntaxKind { text: "FROM", id: 5, is_keyword: true, is_terminal: true };
    #[allow(non_upper_case_globals)] pub static r#SEMI: SyntaxKind = SyntaxKind { text: "SEMI", id: 6, is_keyword: true, is_terminal: true };
    #[allow(non_upper_case_globals)] pub static r#EOF: SyntaxKind = SyntaxKind { text: "EOF", id: 7, is_keyword: true, is_terminal: true };
    #[allow(non_upper_case_globals)] pub static r#distinct: SyntaxKind = SyntaxKind { text: "distinct", id: 8, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#sclp: SyntaxKind = SyntaxKind { text: "sclp", id: 9, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#scanpt: SyntaxKind = SyntaxKind { text: "scanpt", id: 10, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#nm: SyntaxKind = SyntaxKind { text: "nm", id: 11, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#expr: SyntaxKind = SyntaxKind { text: "expr", id: 12, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#as: SyntaxKind = SyntaxKind { text: "as", id: 13, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#selcollist: SyntaxKind = SyntaxKind { text: "selcollist", id: 14, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#stl_prefix: SyntaxKind = SyntaxKind { text: "stl_prefix", id: 15, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#dbnm: SyntaxKind = SyntaxKind { text: "dbnm", id: 16, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#on_using: SyntaxKind = SyntaxKind { text: "on_using", id: 17, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#seltablist: SyntaxKind = SyntaxKind { text: "seltablist", id: 18, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#from: SyntaxKind = SyntaxKind { text: "from", id: 19, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#where_opt: SyntaxKind = SyntaxKind { text: "where_opt", id: 20, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#groupby_opt: SyntaxKind = SyntaxKind { text: "groupby_opt", id: 21, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#having_opt: SyntaxKind = SyntaxKind { text: "having_opt", id: 22, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#orderby_opt: SyntaxKind = SyntaxKind { text: "orderby_opt", id: 23, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#limit_opt: SyntaxKind = SyntaxKind { text: "limit_opt", id: 24, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#oneselect: SyntaxKind = SyntaxKind { text: "oneselect", id: 25, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#selectnowith: SyntaxKind = SyntaxKind { text: "selectnowith", id: 26, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#select: SyntaxKind = SyntaxKind { text: "select", id: 27, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#cmd: SyntaxKind = SyntaxKind { text: "cmd", id: 28, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#cmdx: SyntaxKind = SyntaxKind { text: "cmdx", id: 29, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#ecmd: SyntaxKind = SyntaxKind { text: "ecmd", id: 30, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#cmdlist: SyntaxKind = SyntaxKind { text: "cmdlist", id: 31, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#input: SyntaxKind = SyntaxKind { text: "input", id: 32, is_keyword: false, is_terminal: false };
    #[allow(non_upper_case_globals)] pub static r#program: SyntaxKind = SyntaxKind { text: "program", id: 33, is_keyword: false, is_terminal: false };
}

pub static SYNTAX_KIND_MAP: phf::Map<u32, SyntaxKind> = phf::phf_map! {
    1u32 => syntax_kind::r#SELECT,
    2u32 => syntax_kind::r#ID,
    3u32 => syntax_kind::r#DOT,
    4u32 => syntax_kind::r#COMMA,
    5u32 => syntax_kind::r#FROM,
    6u32 => syntax_kind::r#SEMI,
    7u32 => syntax_kind::r#EOF,
    8u32 => syntax_kind::r#distinct,
    9u32 => syntax_kind::r#sclp,
    10u32 => syntax_kind::r#scanpt,
    11u32 => syntax_kind::r#nm,
    12u32 => syntax_kind::r#expr,
    13u32 => syntax_kind::r#as,
    14u32 => syntax_kind::r#selcollist,
    15u32 => syntax_kind::r#stl_prefix,
    16u32 => syntax_kind::r#dbnm,
    17u32 => syntax_kind::r#on_using,
    18u32 => syntax_kind::r#seltablist,
    19u32 => syntax_kind::r#from,
    20u32 => syntax_kind::r#where_opt,
    21u32 => syntax_kind::r#groupby_opt,
    22u32 => syntax_kind::r#having_opt,
    23u32 => syntax_kind::r#orderby_opt,
    24u32 => syntax_kind::r#limit_opt,
    25u32 => syntax_kind::r#oneselect,
    26u32 => syntax_kind::r#selectnowith,
    27u32 => syntax_kind::r#select,
    28u32 => syntax_kind::r#cmd,
    29u32 => syntax_kind::r#cmdx,
    30u32 => syntax_kind::r#ecmd,
    31u32 => syntax_kind::r#cmdlist,
    32u32 => syntax_kind::r#input,
    33u32 => syntax_kind::r#program,
};

impl cstree::Syntax for SyntaxKind {
    fn from_raw(raw: cstree::RawSyntaxKind) -> Self {
        *SYNTAX_KIND_MAP.get(&raw.0).unwrap()
    }

    fn into_raw(self) -> cstree::RawSyntaxKind {
        cstree::RawSyntaxKind(self.id)
    }

    fn static_text(self) -> Option<&'static str> {
        if self.is_keyword {
            return Some(self.text);
        }
        None
    }
}

enum ParseStatus {
    Shift { kind: SyntaxKind, value: Option<String> },
    Reduce { kind: SyntaxKind, len: usize },
    Accept { kind: SyntaxKind },
}

