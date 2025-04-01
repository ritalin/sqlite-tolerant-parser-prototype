mod lemon;

use std::{env::Args, ffi::{c_int, CStr, CString}, mem::MaybeUninit};

use serde::ser::SerializeStruct;

fn main() -> Result<(), anyhow::Error> {
    let filename = CString::new("submodules/sqlite/src/parse.y").unwrap();

    let mut lem = LemonBuilder::new()
        .set_args(std::env::args())
        .set_grammar(&filename)
        .build()
    ;
    
    lem.parse();
    lem.dump();

    println!("{}", serde_json::to_string_pretty(&lem)?);
    Ok(())
}

struct LemonBuilder {
    inner: lemon::lemon,
}

impl LemonBuilder {
    pub fn new() -> Self {
        let mut inner: lemon::lemon = {
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

struct Lemon {
    inner: lemon::lemon,
}

impl Lemon {
    pub fn from_raw(ptr: lemon::lemon) -> Self {
        Self { inner: ptr }
    }

    pub fn parse(&mut self) {
        unsafe { 
            lemon::Symbol_init();
            lemon::Symbol_new(CString::new("$").unwrap().as_ptr());
            lemon::Parse(&mut self.inner);
        
            lemon::Symbol_new(CString::new("{default}").unwrap().as_ptr());

            self.inner.nsymbol = lemon::Symbol_count();
            self.inner.symbols = lemon::Symbol_arrayof();
        };
    }

    pub fn dump(&mut self) {
        unsafe { lemon::ReportOutput(&mut self.inner) }; 
    }

    pub fn rules(&self) -> Vec<RuleChoice> {
        let mut rule_indexes = vec![];
        let mut processsed = std::collections::HashSet::new();
        let mut rule_raw = self.inner.rule;

        while ! rule_raw.is_null() {
            unsafe {
                let mut members = vec![rule_raw];
                let mut next_lhs = (*rule_raw).nextlhs;

                while !next_lhs.is_null() {
                    let index = (*next_lhs).index;
                    if !processsed.contains(&index) {
                        processsed.insert(index);

                        members.push(next_lhs);
                    }
                    next_lhs = (*next_lhs).nextlhs;
                }
                rule_indexes.push(members);

                rule_raw = (*rule_raw ).next;
            }
        }

        rule_indexes.into_iter()
            .map(|members| RuleChoice::from_raw(&members))
            .collect()
    }
}

impl serde::Serialize for Lemon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("grammar", 2)?;

        let symbols = unsafe { std::slice::from_raw_parts(self.inner.symbols, self.inner.nsymbol as usize) }.into_iter()
            .map(|&x| Symbol::from_raw(x))
            .collect::<Vec<_>>()
        ;

        state.serialize_field("symbols", &symbols)?;
        state.serialize_field("rules", &self.rules())?;
        state.end()
    }
}

#[repr(u32)]
#[derive(serde::Serialize)]
enum SymbolType {
    Terminal,
    NonTerminal,
    MultiTerminal(Vec<String>),
}

#[derive(serde::Serialize)]
enum Precedence {
    Left(c_int),
    Right(c_int),
    None,
}
impl Precedence {
    pub fn from_raw(assoc: lemon::e_assoc, prec: c_int) -> Option<Precedence> {
        match assoc {
            lemon::e_assoc_LEFT => Some(Precedence::Left(prec)),
            lemon::e_assoc_RIGHT => Some(Precedence::Right(prec)),
            lemon::e_assoc_NONE => Some(Precedence::None),
            lemon::e_assoc_UNK => None,
            _ => panic!("Unexpected precedence value (assoc: {assoc}, prec: {prec})"),
        }
    }
}

struct Symbol {
    inner: *mut lemon::symbol,
}

impl Symbol {
    pub fn from_raw(ptr: *mut lemon::symbol) -> Self {
        Self { inner: ptr }
    }

    fn name(&self) -> String {
        unsafe { CStr::from_ptr((*self.inner).name)
        .to_string_lossy()
        .to_string() }
    }

    pub fn symbol_type(&self) -> SymbolType {
        match (unsafe { *self.inner }).type_ {
            lemon::symbol_type_TERMINAL => {
                SymbolType::Terminal
            }
            lemon::symbol_type_NONTERMINAL => {
                SymbolType::NonTerminal
            }
            lemon::symbol_type_MULTITERMINAL => {
                let names = self.additional_names();
                SymbolType::MultiTerminal(names)
            }
            n => panic!("unexpected symbol type value ({n})"),
        }
    }

    fn additional_names(&self) -> Vec<String> {
        unsafe { std::slice::from_raw_parts((*self.inner).subsym, (*self.inner).nsubsym as usize) }.into_iter()
            .map(|&sub| Symbol::from_raw(sub).name())
            .collect()
    }

    pub fn precedence(&self) -> Option<Precedence> {
        Precedence::from_raw((unsafe { *self.inner }).assoc, (unsafe { *self.inner }).prec)
    }
}

impl serde::Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("symbol", 3)?;
        state.serialize_field("name", &self.name())?;
        state.serialize_field("type", &self.symbol_type())?;
        // state.serialize_field("index", &(unsafe { *self.inner }).index)?;
        state.serialize_field("precedence", &self.precedence())?;
        state.end()
    }
}

struct Rule {
    inner: *mut lemon::rule,
}

impl Rule {
    pub fn from_raw(rule: *mut lemon::rule) -> Self {
        Self { inner: rule }
    }

    pub fn rhs(&self) -> Vec<Rhs> {
        let rhs = unsafe { std::slice::from_raw_parts((*self.inner).rhs, (*self.inner).nrhs as usize) };
        let rhs_alias = unsafe { std::slice::from_raw_parts((*self.inner).rhsalias, (*self.inner).nrhs as usize) };

        let rhs = rhs.into_iter().zip(rhs_alias)
            .map(|(&x, &alias)| Rhs::from_raw(x, alias))
            .collect::<Vec<_>>()
        ;

        rhs
    }
}

impl serde::Serialize for Rule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        let mut state = serializer.serialize_struct("rule", 10)?;
        state.serialize_field("index", &(unsafe { *self.inner }).index)?;
        state.serialize_field("choices", &self.rhs())?;
        state.end()
    }
}

#[derive(serde::Serialize)]
struct RuleChoice {
    lhs: String,
    members: Vec<Rule>,
}

impl RuleChoice {
    pub fn from_raw(members: &[*mut lemon::rule]) -> Self {
        Self { 
            lhs: RuleChoice::name_from(members[0]),
            members: members.into_iter().map(|&x| Rule::from_raw(x)).collect() 
        }
    }

    fn name_from(rule: *mut lemon::rule) -> String {
        unsafe { CStr::from_ptr((*(*rule).lhs).name) }
        .to_string_lossy()
        .to_string()
    }
}

#[derive(serde::Serialize)]
struct Rhs {
    name: String,
    alias: Option<String>,
}

impl Rhs {
    pub fn from_raw(symbol: *mut lemon::symbol, alias: *const i8) -> Self {
        let name = unsafe { CStr::from_ptr((*symbol).name) }
            .to_string_lossy()
            .to_string()
        ;
        let alias = match alias.is_null() {
            false => Some(unsafe { CStr::from_ptr(alias) }.to_string_lossy().to_string()),
            true => None,
        };

        Self { name, alias }
    }
}