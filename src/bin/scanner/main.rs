
use std::collections::HashMap;

use regex::Regex;
use sqlite_parser_proto::{Grammar, GrammarSymbol, SymbolType};

pub fn main() -> Result<(), anyhow::Error> {
    let source = r#"
    /* 行頭Comment */
    FROM (
        /* なんたらかんたら */
        SELECT t.*, 'abc' || 'xyz' AS x, 123 / 456 AS y 
        FROM foo t 
        WHERE t.code = 10 -- 条件
    )
    "#;

    let mut scanner = Scanner::create(source)?;

    while let Some(token) = scanner.scan_next() {
        println!("{token:?}");
        println!("--------------------");
    }

    Ok(())
}

struct Scanner<'a> {
    source: &'a str,
    index: usize,
    lexme_scanners: LexmeScannerSet,
    extra_scanners: ExtraScannerSet,
}

impl<'a> Scanner<'a> {
    pub fn create(source: &'a str) -> Result<Self, anyhow::Error> {
        let grammar = serde_json::from_str::<Grammar>(include_str!("../../../build/grammar.json"))?;
        
        let this = Self { 
            source, 
            index: 0,
            lexme_scanners: LexmeScannerSet::new(&grammar.symbols),
            extra_scanners: ExtraScannerSet {
                scanners: vec![
                    ExtraScanner { 
                        re: Regex::new(r"(?s)/\*.*?\*/")?, 
                        tag: TokenTag::ExtraMarginalia,
                    },
                    ExtraScanner { 
                        re: Regex::new("--.*")?, 
                        tag: TokenTag::ExtraMarginalia,
                    },
                    ExtraScanner { 
                        re: Regex::new(r"\s+")?, 
                        tag: TokenTag::ExtraWhitespace,
                    },
                    ExtraScanner { 
                        re: Regex::new(r#"(x|X)'.*?'"#)?, 
                        tag: TokenTag::BlobLiteral,
                    },
                    ExtraScanner { 
                        re: Regex::new(r#"'.*?'"#)?, 
                        tag: TokenTag::StringLiteral,
                    },
                    ExtraScanner { 
                        re: Regex::new(r#"".*?""#)?, 
                        tag: TokenTag::Identifier,
                    },
                    ExtraScanner { 
                        re: Regex::new(r"(x|X)(\d+(_\d+)*)")?, 
                        tag: TokenTag::HexLiteral,
                    },
                    ExtraScanner { 
                        re: Regex::new(r"((\d+(_\d+)*)?[.]\d+(_\d+)*(e[+-]?\d+(_\d+)*)?)|(\d+(_\d+)*[.](e[+-]?\d+(_\d+)*)?)")?, 
                        tag: TokenTag::NumberLiteral,
                    },
                    ExtraScanner { 
                        re: Regex::new(r"(\d+(_\d+)*)")?, 
                        tag: TokenTag::NumberLiteral,
                    },
                    ExtraScanner { 
                        re: Regex::new(r"[a-zA-Z_][0-9a-zA-Z_]*")?, 
                        tag: TokenTag::Identifier,
                    },
                ],
                support_leading: vec![0, 1, 2], 
                support_trailing: vec![2], 
                support_main: vec![3, 4, 5, 6, 7, 8, 9] 
            }
        };

        Ok(this)
    }

    pub fn scan_next(&mut self) -> Option<Token> {
        let mut index = self.index;

        let leading = self.scan_extra(index, &self.extra_scanners.support_leading);
        if let Some(item) = leading.as_ref() {
            if let Some(last) = item.last() {
                index = last.range.end;
            }
        }

        let main = match self.scan_main(index, &self.extra_scanners.support_main) {
            Some(item) if item.tag == TokenTag::Eof => {
                index = item.range.end + 1;
                item
            }
            Some(item) => {
                index = item.range.end;
                item
            }
            None => {
                self.index = index;
                return None;
            }
        };
        let trailing = self.scan_extra(index, &self.extra_scanners.support_trailing);
        if let Some(item) = trailing.as_ref() {
            if let Some(last) = item.last() {
                index = last.range.end;
            }
        }
        self.index = index;

        Some(Token {
            leading,
            main,
            trailing,
        })
    }

    fn scan_extra(&self, mut index: usize, extra_scanners: &[usize]) -> Option<Vec<TokenItem>> {
        let mut items = vec![];


        while let Some(offset) = self.scan_extra_internal(index, extra_scanners, false, &mut items) {
            index += offset;
        }

        (!items.is_empty()).then(|| items)
    }

    fn scan_extra_internal(&self, index: usize, extra_scanners: &[usize], oneshot: bool, items: &mut Vec<TokenItem>) -> Option<usize> {
        let mut offset = 0;

        for &i in extra_scanners {
            let Some(source) = self.source.get((index + offset)..) else {
                break;
            };

            let item = self.extra_scanners.scanners[i].scan(source, index + offset);
            if let Some(item) = item {
                offset = item.range.len();
                items.push(item);

                if oneshot {
                    return Some(offset);
                }
            }
        }

        (offset > 0).then(|| offset)
    }

    fn scan_main(&self, index: usize, extra_scanners: &[usize]) -> Option<TokenItem> {
        if self.source.len() < self.index {
            return None;
        }

        if self.source.len() == self.index {
            return Some(TokenItem { range: index..index, tag: TokenTag::Eof });
        }

        if let Some(source) = self.source.get(index..) {
            let item = self.lexme_scanners.scan(source, index);
            if item.is_some() {
                return item;
            }

            let mut items = vec![];
            if self.scan_extra_internal(index, extra_scanners, true, &mut items).is_some() {
                return items.first().cloned();
            }
        }

        let next_index = self.source.get(index..)
            .and_then(|s| s.char_indices().skip(1).map(|(i, _)| i).next())
            .unwrap_or(1)
        ;

        Some(TokenItem { 
            range: index..(index + next_index), 
            tag: TokenTag::Illegal
        })
    }
}

struct LexmeScannerSet {
    map: HashMap<char, Vec<LexmeScanner>>,
}

impl LexmeScannerSet {
    fn new(symbols: &[GrammarSymbol]) -> Self {
        let scanners = symbols.iter()
            .filter_map(|symbol| match symbol.symbol_type {
                SymbolType::Terminal { is_keyword } if is_keyword => {
                    Some(LexmeScanner {
                        value: symbol.name.clone(),
                        tag: TokenTag::Keyword(symbol.name.clone()),
                    })
                },
                _ => None,
            })
        ;

        let mut map = HashMap::<char, Vec<LexmeScanner>>::new();

        for scanner in scanners {
            if let Some(c) = scanner.value.chars().nth(0) {
                map.entry(c.to_ascii_uppercase())
                    .and_modify(|v| {
                        v.push(scanner.clone());
                    })
                    .or_insert_with(|| vec![scanner])
                ;
            }
        }

        LexmeScannerSet::create_additional_scanners(&mut map);

        for value in map.values_mut() {
            value.sort_by(|lhs, rhs| rhs.value.len().cmp(&lhs.value.len()));
        }

        Self { map }
    }

    fn create_additional_scanners(map: &mut HashMap<char, Vec<LexmeScanner>>) {
    // "CTIME_KW", // (?) CURRENT_DATE / CURRENT_TIME / CURRENT_TIMESTAMP
    // "COLUMNKW", // (?) COLUMN
    // "LIKE_KW", // (?) LIKE / GLOB / REGEXP
    // "JOIN_KW", // (?) CROSS / FULL / INNER LEFT / NATURAL / OUTER / RIGHT
    // "TRUEFALSE", (?) TRUE / FALSE
    // "SEMI", // ; (*)
    // "LP", // ( (*)
    // "RP", // ) (*)
    // "NE",
    // "EQ",
    // "GT",
    // "LE",
    // "LT",
    // "GE",
    // "BITAND", `&`
    // "BITOR", `|`
    // "LSHIFT",
    // "RSHIFT",
    // "PLUS", | "UPLUS",
    // "MINUS", // - (*) | "UMINUS",
    // "SLASH",
    // "REM", // `%` (*)
    // "PTR", // `->` (*)
    // "BITNOT", // `~` (*)
    // "STRING", (*)
    // "DOT",
    // "VARIABLE", // `?` (*)
    // "ASTERISK", `*`
        let symbols = vec![
            ("CTIME_KW", vec!["CURRENT_DATE", "CURRENT_TIME", "CURRENT_TIMESTAMP"]),
            ("COLUMNKW", vec!["COLUMN"]),
            ("LIKE_KW", vec!["LIKE", "GLOB", "REGEXP"]),
            ("JOIN_KW", vec!["CROSS", "FULL", "INNER", "LEFT", "NATURAL", "OUTER", "RIGHT"]),
            ("TRUEFALSE", vec!["TRUE", "FALSE"]),
            ("SEMI", vec![";"]),
            ("LP", vec!["("]),
            ("RP", vec![")"]),
            ("NE", vec!["<>", "!="]),
            ("EQ", vec!["="]),
            ("GT", vec![">"]),
            ("LE", vec!["<="]),
            ("LT", vec!["<"]),
            ("GE", vec![">="]),
            ("BITAND", vec!["&"]),
            ("BITOR", vec!["|"]),
            ("LSHIFT", vec!["<<"]),
            ("RSHIFT", vec![">>"]),
            ("PLUS", vec!["+"]), // FIXME: "UPLUS",
            ("MINUS",  vec!["-"]), // FIXME: "UMINUS",
            ("SLASH", vec!["/"]),
            ("REM", vec!["%"]),
            ("PTR", vec!["->"]), 
            ("BITNOT", vec!["~"]), 
            ("DOT", vec!["."]),
            ("VARIABLE", vec!["?"]),
            ("ASTERISK", vec!["*"]),
            ("CONCAT", vec!["||"]),
            ("COMMA", vec![","]),
        ];

        for (symbol, needles) in symbols {
            for needle in needles {
                if let Some(c) = needle.chars().nth(0) {
                    let scanner = LexmeScanner { 
                        value: needle.into(), 
                        tag: TokenTag::Keyword(symbol.into()),
                    };

                    map.entry(c.to_ascii_uppercase())
                        .and_modify(|v| {
                            v.push(scanner.clone());
                        })
                        .or_insert_with(|| vec![scanner])
                    ;
                }
            }
        }
    }

    pub fn scan(&self, source: &str, index: usize) -> Option<TokenItem> {
        let Some(c) = source.chars().nth(0) else {
            return Some(TokenItem { 
                range: index..index+1, 
                tag: TokenTag::Illegal,
            });
        };
        let Some(scanners) = self.map.get(&c.to_ascii_uppercase()) else {
            return None;
        };
    
        for scanner in scanners {
            if let Some(token) = scanner.scan(source, index) {
                return Some(token);
            }
        }

        None
    }
}

#[derive(Clone)]
struct LexmeScanner {
    value: String,
    tag: TokenTag,
}

impl LexmeScanner {
    pub fn scan(&self, source: &str, index: usize) -> Option<TokenItem> {
        if let Some(s) = source.get(..self.value.len()) {
            if s.eq_ignore_ascii_case(self.value.as_str()) {
                return Some(TokenItem { 
                    range: index..(index+self.value.len()), 
                    tag: self.tag.clone(),
                });
            }
        }

        None
    }
}

struct ExtraScannerSet {
    scanners: Vec<ExtraScanner>,
    support_leading: Vec<usize>,
    support_trailing: Vec<usize>,
    support_main: Vec<usize>,
}

#[derive(PartialEq, Clone, Debug)]
pub enum TokenTag {
    // "TRUTH", // (?) IS TRUE / IS FALSE / IS NOT TRUE / IS NOT FALSE
    // "ISNOT", // (x) `IS NOT`` 

    // "ANY", (unused)
    // "SPAN", (unused)
    // "VECTOR", (unused)
    // "SELECT_COLUMN", (unused)
    // "REGISTER",(unused)
    // "AGG_FUNCTION",
    // "AGG_COLUMN",
    // "FUNCTION",    
    // "IF_NULL_ROW",

    // "CTIME_KW", // (?) CURRENT_DATE / CURRENT_TIME / CURRENT_TIMESTAMP
    // "COLUMNKW", // (?) COLUMN
    // "LIKE_KW", // (?) LIKE / GLOB / REGEXP
    // "JOIN_KW", // (?) CROSS / FULL / INNER LEFT / NATURAL / OUTER / RIGHT
    // "TRUEFALSE", (?) TRUE / FALSE
    // "SEMI", // ; (*)
    // "LP", // ( (*)
    // "RP", // ) (*)
    // "NE",
    // "EQ",
    // "GT",
    // "LE",
    // "LT",
    // "GE",
    // "BITAND", `&`
    // "BITOR", `|`
    // "LSHIFT",
    // "RSHIFT",
    // "PLUS", | "UPLUS",
    // "MINUS", // - (*) | "UMINUS",
    // "SLASH",
    // "REM", // `%` (*)
    // "PTR", // `->` (*)
    // "BITNOT", // `~` (*)
    // "STRING", (*)
    // "DOT",
    // "VARIABLE", // `?` (*)
    // "ASTERISK", `*`

    // "AUTOINCR",
    Keyword(String),
    
    // "INTEGER", (*)
    HexLiteral, // "QNUMBER", // hex int
    NumberLiteral,
    BlobLiteral,
    StringLiteral,// "STRING" / "BLOB"
    Identifier, // "ID", *
    Illegal,
    Eof,
    ExtraSpace,
    ExtraMarginalia,
    ExtraWhitespace, // "SPACE", (*)
}

#[derive(Clone, Debug)]
pub struct TokenItem {
    range: std::ops::Range<usize>,
    tag: TokenTag,
}

#[derive(Debug)]
pub struct Token {
    pub leading: Option<Vec<TokenItem>>,
    pub main: TokenItem,
    pub trailing: Option<Vec<TokenItem>>,
}

struct ExtraScanner {
    re: Regex,
    tag: TokenTag,
}

impl ExtraScanner {
    pub fn scan(&self, source: &str, index: usize) -> Option<TokenItem> {
        match self.re.find_at(source, 0) {
            Some(m) if m.start() == 0 => {
                Some(TokenItem { 
                    range: index..(index + m.len()), 
                    tag: self.tag.clone(), 
                })
            }
            _ => None
        }
    }
}