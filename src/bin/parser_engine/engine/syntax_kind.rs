use super::SyntaxKind;

#[allow(non_upper_case_globals)] pub static r#SELECT: SyntaxKind = SyntaxKind { text: "SELECT", id: 201, is_keyword: true, is_terminal: true };
#[allow(non_upper_case_globals)] pub static r#ID: SyntaxKind = SyntaxKind { text: "ID", id: 85, is_keyword: false, is_terminal: true };
#[allow(non_upper_case_globals)] pub static r#DOT: SyntaxKind = SyntaxKind { text: "DOT", id: 218, is_keyword: true, is_terminal: true };
#[allow(non_upper_case_globals)] pub static r#COMMA: SyntaxKind = SyntaxKind { text: "COMMA", id: 47, is_keyword: true, is_terminal: true };
#[allow(non_upper_case_globals)] pub static r#FROM: SyntaxKind = SyntaxKind { text: "FROM", id: 221, is_keyword: true, is_terminal: true };
#[allow(non_upper_case_globals)] pub static r#SEMI: SyntaxKind = SyntaxKind { text: "SEMI", id: 5, is_keyword: true, is_terminal: true };
#[allow(non_upper_case_globals)] pub static r#EOF: SyntaxKind = SyntaxKind { text: "EOF", id: 191, is_keyword: true, is_terminal: true };
#[allow(non_upper_case_globals)] pub static r#distinct: SyntaxKind = SyntaxKind { text: "distinct", id: 202, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#sclp: SyntaxKind = SyntaxKind { text: "sclp", id: 216, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#scanpt: SyntaxKind = SyntaxKind { text: "scanpt", id: 152, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#nm: SyntaxKind = SyntaxKind { text: "nm", id: 16, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#expr: SyntaxKind = SyntaxKind { text: "expr", id: 158, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#as: SyntaxKind = SyntaxKind { text: "as", id: 217, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#selcollist: SyntaxKind = SyntaxKind { text: "selcollist", id: 203, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#stl_prefix: SyntaxKind = SyntaxKind { text: "stl_prefix", id: 220, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#dbnm: SyntaxKind = SyntaxKind { text: "dbnm", id: 33, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#on_using: SyntaxKind = SyntaxKind { text: "on_using", id: 223, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#seltablist: SyntaxKind = SyntaxKind { text: "seltablist", id: 219, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#from: SyntaxKind = SyntaxKind { text: "from", id: 204, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#where_opt: SyntaxKind = SyntaxKind { text: "where_opt", id: 205, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#groupby_opt: SyntaxKind = SyntaxKind { text: "groupby_opt", id: 206, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#having_opt: SyntaxKind = SyntaxKind { text: "having_opt", id: 207, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#orderby_opt: SyntaxKind = SyntaxKind { text: "orderby_opt", id: 208, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#limit_opt: SyntaxKind = SyntaxKind { text: "limit_opt", id: 209, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#oneselect: SyntaxKind = SyntaxKind { text: "oneselect", id: 194, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#selectnowith: SyntaxKind = SyntaxKind { text: "selectnowith", id: 193, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#select: SyntaxKind = SyntaxKind { text: "select", id: 45, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#cmd: SyntaxKind = SyntaxKind { text: "cmd", id: 11, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#cmdx: SyntaxKind = SyntaxKind { text: "cmdx", id: 6, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#ecmd: SyntaxKind = SyntaxKind { text: "ecmd", id: 4, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#cmdlist: SyntaxKind = SyntaxKind { text: "cmdlist", id: 3, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#input: SyntaxKind = SyntaxKind { text: "input", id: 2, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#program: SyntaxKind = SyntaxKind { text: "program", id: 329, is_keyword: false, is_terminal: false };
#[allow(non_upper_case_globals)] pub static r#error: SyntaxKind = SyntaxKind { text: "ERROR", id: 4294967295, is_keyword: false, is_terminal: false };

pub mod maps {
    use super::*;
    
    pub static SYNTAX_KIND_MAP: phf::Map<u32, SyntaxKind> = phf::phf_map! {
        1u32 => r#SELECT,
        2u32 => r#ID,
        3u32 => r#DOT,
        4u32 => r#COMMA,
        5u32 => r#FROM,
        6u32 => r#SEMI,
        7u32 => r#EOF,
        8u32 => r#distinct,
        9u32 => r#sclp,
        10u32 => r#scanpt,
        11u32 => r#nm,
        12u32 => r#expr,
        13u32 => r#as,
        14u32 => r#selcollist,
        15u32 => r#stl_prefix,
        16u32 => r#dbnm,
        17u32 => r#on_using,
        18u32 => r#seltablist,
        19u32 => r#from,
        20u32 => r#where_opt,
        21u32 => r#groupby_opt,
        22u32 => r#having_opt,
        23u32 => r#orderby_opt,
        24u32 => r#limit_opt,
        25u32 => r#oneselect,
        26u32 => r#selectnowith,
        27u32 => r#select,
        28u32 => r#cmd,
        29u32 => r#cmdx,
        30u32 => r#ecmd,
        31u32 => r#cmdlist,
        32u32 => r#input,
        33u32 => r#program,
        4294967295u32 => r#error
    };
}
