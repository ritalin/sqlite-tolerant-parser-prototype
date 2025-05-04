use std::ffi::CString;
use tools::binding::lemon::{self, SymbolType};

pub fn main() {
    let filename = CString::new("submodules/sqlite/src/parse.y").unwrap();

    let mut lem = lemon::LemonBuilder::new()
        .set_args(std::env::args())
        .set_grammar(&filename)
        .build()
    ;
    
    lem.parse();

    let prefix = lem.symbol_prefix();

    let iter = lem.symbols()
        .skip(1)
        .filter(|x| match x.symbol_type() {
            SymbolType::Terminal{..} => true,
            _ => false,
        }).enumerate()
    ;

    for (i, symbol) in iter {
        let name = symbol.name();
        if name != "{default}" {
            println!("#define {}{} {}", prefix, symbol.name(), i +1);
        }
    }
}