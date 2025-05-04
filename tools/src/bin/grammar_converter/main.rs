use tools::binding::lemon::LemonBuilder;
use std::ffi::CString;

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
