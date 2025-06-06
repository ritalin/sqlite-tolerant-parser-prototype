use sqlite_parser_proto::engine;
use sqlite_parser_proto::engine::RegexScanPattern;
use sqlite_parser_proto::SyntaxKind;

use crate::{Token, TokenItem};

pub struct Scanner {
    source: String,
    index: usize,
    lookahead: Option<Token>,
}

impl Scanner {
    pub fn create(source: String, index_from: usize) -> Self {
        let mut this = Self {
            source,
            index: index_from,
            lookahead: None,
        };
        this.shift();
        this
    }

    pub fn shift(&mut self) -> Option<Token> {
        let lookahead = self.lookahead.take();
        self.lookahead = self.scan_next();

        lookahead
    }

    pub fn lookahead(&self) -> Option<&Token> {
        self.lookahead.as_ref()
    }

    fn scan_next(&mut self) -> Option<Token> {
        let mut index = self.index;
        let mut leading = None;
        let mut trailing = None;

        if let Some((next_index, item)) = scan_extra(&self.source, index, engine::support_leading()) {
            index = next_index;
            leading = Some(item);
        }

        let main = match scan_main(&self.source, index, engine::support_main()) {
            Some((next_index, item)) => {
                index = next_index;
                item
            }
            None => {
                self.index = index;
                return None;
            }
        };        

        if let Some((next_index, item)) = scan_extra(&self.source, index, engine::support_trailing()) {
            index = next_index;
            trailing = Some(item);
        }
        self.index = index;

        Some(Token { leading, main, trailing })
    }

    pub fn scope(&self) -> ScannerScope {
        ScannerScope {
            next_index: self.index,
            lookahead: self.lookahead.clone(),
        }
    }

    pub fn revert(&mut self, scope: ScannerScope) {
        self.index = scope.next_index;
        self.lookahead = scope.lookahead;
    }
}

fn scan_extra(source: &str, mut index: usize, extra_scanners: &[usize]) -> Option<(usize, Vec<TokenItem>)> {
    let mut items = vec![];

    let scanners = engine::regex_scan_patterns(extra_scanners);

    while let Some(next_index) = scan_extra_internal(source, index, &scanners, &mut items) {
        index = next_index;
    }

    (!items.is_empty()).then(|| (index, items))
}

fn scan_extra_internal(source: &str, index: usize, scanners: &[&RegexScanPattern], items: &mut Vec<TokenItem>) -> Option<usize> {
    let Some(source) = source.get((index)..) else {
        return None;
    };

    let mut offset = 0;

    for scanner in scanners {
        if let Some(item) = scan_regex(scanner, source, index + offset) {
            offset = item.len;
            items.push(item);
            break
        }
    }

    (offset > 0).then(|| index + offset)
}

fn scan_regex(scanner: &RegexScanPattern, source: &str, index: usize) -> Option<TokenItem> {
    match scanner.pattern.find_at(source, 0) {
        Some(m) if m.start() == 0 => {
            Some(TokenItem { 
                tag: scanner.kind.clone(), 
                offset: index,
                len: m.len(), 
                value: Some(m.as_str().to_string())
            })
        }
        _ => None
    }
}

fn scan_main(source: &str, index: usize, extra_scanners: &[usize]) -> Option<(usize, TokenItem)> {
    use cstree::Syntax;
    if source.len() < index {
        return None;
    }
    if source.len() == index {
        let item = TokenItem { tag: engine::kinds::r#EOF, offset: index, len: 0, value: None };
        return Some((index + 1, item));
    }

    if let Some(sub_source) = source.get(index..) {
        let item1 = engine::scan_by_lexme_rule(sub_source).and_then(|item| {
            let tag = SyntaxKind::from_raw(cstree::RawSyntaxKind(item.id));
            let item = TokenItem { 
                tag, 
                offset: index,
                len: item.len, 
                value: Some(item.pattern.to_string())
            };
            Some((item.offset + item.len, item))
        });
        let item2 = {
            let mut items = vec![];
            let scanners = engine::regex_scan_patterns(extra_scanners);
            scan_extra_internal(source, index, &scanners, &mut items).and_then(|next_index| {
                items.first().map(|item| (next_index, item.clone()))
            })
        };

        match (item1.as_ref(), item2.as_ref()) {
            (Some((_, token1)), Some((_, token2))) => {
                return if token1.len < token2.len { item2 } else { item1 };
            }
            (Some(_), None) => {
                return item1;
            }
            (None, Some(_)) => {
                return item2;
            }
            (None, None) => {
            }
        }
    }

    let (len, illegal_char) = source.get(index..)
        .and_then(|s| s.char_indices().next())
        .unwrap_or((1, '\0'))
    ;

    let item = TokenItem { 
        tag: engine::kinds::r#ILLEGAL,
        offset: index,
        len,
        value: Some(illegal_char.to_string()),
    };
    Some((item.offset + item.len, item))
}

#[derive(Clone)]
pub struct ScannerScope {
    pub next_index: usize,
    pub lookahead: Option<Token>,
}
