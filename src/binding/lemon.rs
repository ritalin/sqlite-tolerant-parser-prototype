use std::{env::Args, ffi::{CStr, CString}, mem::MaybeUninit};
use crate::{Precedence, Term};

use super::lemon_binding;
use super::keyword_check::sqlite3_keyword_check;

pub struct LemonBuilder {
    inner: lemon_binding::lemon,
}

impl LemonBuilder {
    pub fn new() -> Self {
        let mut inner: lemon_binding::lemon = {
            let lem = MaybeUninit::zeroed();
            unsafe { lem.assume_init() }
        };
        inner.errorcnt = 0;
        inner.basisflag = 0;
        inner.nolinenosflag = 0;

        Self { inner }
    }

    pub fn set_args(mut self, args: Args) -> Self {
        let args = args.map(|arg| CString::new(arg).unwrap())
            .collect::<Vec<_>>()
        ;
        let mut c_args: Vec<*mut i8> = args.iter()
            .map(|arg| arg.as_ptr() as *mut i8)
            .collect()
        ;
        c_args.push(std::ptr::null_mut());

        self.inner.argc = c_args.len() as i32;
        self.inner.argv = c_args.as_mut_ptr();

        self
    }

    pub fn set_grammar(mut self, grammar_path: &CString) -> Self {
        self.inner.filename = grammar_path.as_ptr() as *mut i8;

        self
    }

    pub fn build(self) -> Lemon {
        Lemon::from_raw(self.inner)
    }
}

pub struct Lemon {
    inner: lemon_binding::lemon,
}

impl Lemon {
    pub fn from_raw(ptr: lemon_binding::lemon) -> Self {
        Self { inner: ptr }
    }

    pub fn parse(&mut self) {
        unsafe { 
            lemon_binding::Symbol_init();
            lemon_binding::Symbol_new(CString::new("$").unwrap().as_ptr());
            lemon_binding::Parse(&mut self.inner);
        
            lemon_binding::Symbol_new(CString::new("{default}").unwrap().as_ptr());

            self.inner.nsymbol = lemon_binding::Symbol_count();
            self.inner.symbols = lemon_binding::Symbol_arrayof();
        };
    }

    pub fn dump(&mut self) {
        unsafe { lemon_binding::ReportOutput(&mut self.inner) }; 
    }
}

impl Lemon {
    pub fn token_classes(&self) -> Vec<String> {
        self.symbols().into_iter()
            .filter(|sym|match sym.symbol_type() {
                SymbolType::Terminal { is_keyword } if ! is_keyword => true,
                _ => false,
            })
            .map(|sym| sym.name())
            .collect::<Vec<_>>()
    }

    pub fn symbol_prefix(&self) -> String {
        unsafe { CStr::from_ptr(self.inner.tokenprefix) }.to_string_lossy().to_string()
    }

    pub fn symbols(&self) -> impl Iterator<Item = Symbol> {
        unsafe { std::slice::from_raw_parts(self.inner.symbols, self.inner.nsymbol as usize) }.into_iter()
        .enumerate()
        .map(|(i, &x)| Symbol::from_raw(i + 1, x))
    }

    pub fn rules(&self) -> Vec<Rule> {
        let mut rule_map = std::collections::BTreeMap::<i32, Vec<*mut lemon_binding::rule>>::new();
        let mut rule_raw = self.inner.rule;

        while ! rule_raw.is_null() {
            unsafe {
                let mut next_lhs = (*rule_raw).nextlhs;
                let mut index = (*rule_raw).index;

                while !next_lhs.is_null() {
                    index = (*next_lhs).index;
                    next_lhs = (*next_lhs).nextlhs;
                }
                rule_map.entry(index)
                    .and_modify(|members| members.push(rule_raw))
                    .or_insert_with(|| vec![rule_raw])
                ;

                rule_raw = (*rule_raw ).next;
            }
        }

        rule_map.values().into_iter()
            .map(|members| Rule::from_raw(&members))
            .collect::<Vec<_>>()
    }

    fn root_rule(&self) -> String {
        Rule::name_from(self.inner.rule)
    }
}

impl serde::Serialize for Lemon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("grammar", 3)?;

        // state.serialize_field("classes", &self.token_classes())?;
        state.serialize_field("start", &self.root_rule())?;
        state.serialize_field("symbols", &self.symbols().collect::<Vec<_>>())?;
        state.serialize_field("rules", &self.rules())?;
        state.end()
    }
}

#[derive(PartialEq, serde::Serialize)]
pub enum SymbolType {
    Terminal{ is_keyword: bool },
    NonTerminal,
    MultiTerminal{ classes: Vec<String>},
}

pub struct Symbol {
    id: usize,
    inner: *mut lemon_binding::symbol,
}

impl Symbol {
    pub fn from_raw(id: usize, ptr: *mut lemon_binding::symbol) -> Self {
        Self { id, inner: ptr }
    }

    pub fn name(&self) -> String {
        unsafe { CStr::from_ptr((*self.inner).name)
        .to_string_lossy()
        .to_string() }
    }

    pub fn symbol_type(&self) -> SymbolType {
        match (unsafe { *self.inner }).type_ {
            lemon_binding::symbol_type_TERMINAL => {
                let name = self.name().to_lowercase();
                let name_len = name.len();

                let is_keyword = unsafe { sqlite3_keyword_check(CString::new(name).unwrap().as_ptr(), name_len as i32) };
                SymbolType::Terminal { is_keyword: is_keyword == 1 }
            }
            lemon_binding::symbol_type_NONTERMINAL => {
                SymbolType::NonTerminal
            }
            lemon_binding::symbol_type_MULTITERMINAL => {
                let classes = self.additional_names();
                SymbolType::MultiTerminal{ classes }
            }
            n => panic!("unexpected symbol type value ({n})"),
        }
    }

    fn additional_names(&self) -> Vec<String> {
        unsafe { std::slice::from_raw_parts((*self.inner).subsym, (*self.inner).nsubsym as usize) }.into_iter()
            .map(|&sub| Symbol::from_raw(0, sub).name())
            .collect()
    }

    pub fn precedence(&self) -> Option<Precedence> {
        precedence_from_raw((*self).inner)
    }
}

impl serde::Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("symbol", 4)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name())?;
        state.serialize_field("type", &self.symbol_type())?;
        // state.serialize_field("index", &(unsafe { *self.inner }).index)?;
        state.serialize_field("precedence", &self.precedence())?;
        state.end()
    }
}

#[derive(serde::Serialize)]
pub struct Rule {
    lhs: String,
    members: Vec<RuleMember>,
}

impl Rule {
    pub fn from_raw(members: &[*mut lemon_binding::rule]) -> Self {
        let lhs = Rule::name_from(members[0]);
        let mut members = members.into_iter().map(|&x| RuleMember::from_raw(x)).collect::<Vec<_>>();
        members.sort_by(|m1, m2| m1.index.cmp(&m2.index));

        Self { lhs, members }
    }

    pub fn name_from(rule: *mut lemon_binding::rule) -> String {
        unsafe { CStr::from_ptr((*(*rule).lhs).name) }
        .to_string_lossy()
        .to_string()
    }
}

pub struct RuleMember {
    index: usize,
    inner: *mut lemon_binding::rule,
}

impl RuleMember {
    pub fn from_raw(rule: *mut lemon_binding::rule) -> Self {
        Self { 
            index: unsafe {(*rule).index} as usize,
            inner: rule 
        }
    }

    pub fn rhs(&self) -> Vec<crate::Rhs> {
        let rhs = unsafe { std::slice::from_raw_parts((*self.inner).rhs, (*self.inner).nrhs as usize) };
        let rhs_alias = unsafe { std::slice::from_raw_parts((*self.inner).rhsalias, (*self.inner).nrhs as usize) };

        let rhs = rhs.into_iter().zip(rhs_alias)
            .map(|(&x, &alias)| RuleMember::rhs_from_raw(x, alias))
            .collect::<Vec<_>>()
        ;

        rhs
    }

    fn rhs_from_raw(symbol: *mut lemon_binding::symbol, alias: *const i8) -> crate::Rhs {
        let name = unsafe { CStr::from_ptr((*symbol).name) }
            .to_string_lossy()
            .to_string()
        ;
        let alias = match alias.is_null() {
            false => Some(unsafe { CStr::from_ptr(alias) }.to_string_lossy().to_string()),
            true => None,
        };

        unsafe { 
            let sym = *symbol;
            if (sym.type_ == 2) && (sym.useCnt == 0)  {
                return crate::Rhs { token: Term::CharClass { members: Symbol::from_raw(0, symbol).additional_names() }, alias };
            }
        }

        crate::Rhs { token: Term::Symbol {name}, alias }
    }

    pub fn precedence(&self) -> Option<Precedence> {
        unsafe {
            match (*self.inner).precsym.is_null() {
                false => {
                    precedence_from_raw((*self.inner).precsym)
                }
                true => None
            }
        }
    }
}

impl serde::Serialize for RuleMember {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("rule", 10)?;
        state.serialize_field("id", &(self.index + 1))?;
        state.serialize_field("sequences", &self.rhs())?;
        state.serialize_field("precedence", &self.precedence())?;
        state.end()
    }
}

fn precedence_from_raw(sym: *mut lemon_binding::symbol) -> Option<Precedence> {
    match unsafe { ((*sym).assoc, (*sym).prec) } {
        (lemon_binding::e_assoc_LEFT, prec) => Some(Precedence::Left(prec)),
        (lemon_binding::e_assoc_RIGHT, prec) => Some(Precedence::Right(prec)),
        (lemon_binding::e_assoc_NONE, _) => Some(Precedence::Noassoc),
        (lemon_binding::e_assoc_UNK, _) => None,
        (assoc, prec) => panic!("Unexpected precedence value (assoc: {assoc}, prec: {prec})"),
    }
}
