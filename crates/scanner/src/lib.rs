use sqlite_parser_proto::engine;
use sqlite_parser_proto::engine::RegexScanPattern;
use sqlite_parser_proto::SyntaxKind;

pub struct Scanner<'a> {
    source: &'a str,
    index: usize,
    lookahead: Option<Token>,
}

impl<'a> Scanner<'a> {
    pub fn create(source: &'a str) -> Result<Self, anyhow::Error> {
        let this = Self {
            source,
            index: 0,
            lookahead: None,
        };
        Ok(this)
    }

    pub fn shift(&mut self) -> Option<Token> {
        self.lookahead.take()
    }

    pub fn lookahead(&mut self) -> Result<Option<&Token>, anyhow::Error> {
        if self.lookahead.is_none() {
            self.lookahead = self.scan_next()?;
        }

        Ok(self.lookahead.as_ref())
    }

    fn scan_next(&mut self) -> Result<Option<Token>, anyhow::Error> {
        let mut index = self.index;
        let mut leading = None;
        let mut trailing = None;

        if let Some((next_index, item)) = scan_extra(self.source, index, engine::support_leading()) {
            index = next_index;
            leading = Some(item);
        }

        let main = match scan_main(self.source, index, engine::support_main())? {
            Some((next_index, item)) => {
                index = next_index;
                item
            }
            None => {
                self.index = index;
                return Ok(None);
            }
        };        

        if let Some((next_index, item)) = scan_extra(self.source, index, engine::support_trailing()) {
            index = next_index;
            trailing = Some(item);
        }
        self.index = index;

        Ok(Some(Token { leading, main, trailing }))
    }
}

#[derive(Debug)]
pub struct Token {
    pub leading: Option<Vec<TokenItem>>,
    pub main: TokenItem,
    pub trailing: Option<Vec<TokenItem>>,
}

#[derive(Clone, Debug)]
pub struct TokenItem {
    pub tag: SyntaxKind,
    pub offset: usize,
    pub len: usize,
    pub value: Option<String>,
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

fn scan_main(source: &str, index: usize, extra_scanners: &[usize]) -> Result<Option<(usize, TokenItem)>, anyhow::Error> {
    use cstree::Syntax;
    if source.len() < index {
        return Ok(None);
    }
    if source.len() == index {
        let item = TokenItem { tag: engine::kinds::r#EOF, offset: index, len: 0, value: None };
        return Ok(Some((index + 1, item)));
    }

    if let Some(sub_source) = source.get(index..) {
        match engine::scan_by_lexme_rule(sub_source) {
            Some(item) => {
                let tag = SyntaxKind::from_raw(cstree::RawSyntaxKind(item.id));
                let item = TokenItem { 
                    tag, 
                    offset: index,
                    len: item.len, 
                    value: Some(tag.text.to_string())
                };
                return Ok(Some((item.offset + item.len, item)));
            }
            None => {
                let mut items = vec![];
                let scanners = engine::regex_scan_patterns(extra_scanners);
                
                if let Some(next_index) = scan_extra_internal(source, index, &scanners, &mut items) {
                    return Ok(items.first().map(|item| (next_index, item.clone())));
                }
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
    Ok(Some((item.offset + item.len, item)))
}