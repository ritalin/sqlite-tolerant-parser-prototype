use generate::{export_parser_state_pretty, export_scan_rule_pretty, export_syntax_kind_pretty, LalryBuilder, ScanRuleSet};
use sqlite_parser_proto::Grammar;
use std::{
    collections::HashMap,
    io::{BufWriter, Write},
};

pub fn main() -> Result<(), anyhow::Error> {
    let gramer_rule = serde_json::from_str::<Grammar>(include_str!("../../../../build/grammar.json"))?;
    let scanner_rule = serde_json::from_str::<ScanRuleSet>(include_str!("../assets/scan_rule.json"))?;

    let combination_rules = HashMap::<String, (String, Vec<String>)>::from_iter(vec![(
        "IS".into(),
        ("ISNOT".into(), vec!["NOT".into()]),
    )]);
    let start_symbol = "program";

    let builder = LalryBuilder::new(&gramer_rule);
    let grammar = builder.create_lalry_grammar(&gramer_rule, &combination_rules, start_symbol);
    let state_machine = builder.convert_to_lalr(&grammar)?;

    let lookup = HashMap::<String, u32>::from_iter(gramer_rule.symbols.iter().map(|x| (x.name.clone(), x.id)));

    let exported_kinds = export_syntax_kind_pretty(&gramer_rule.symbols);
    let export_states = export_parser_state_pretty(&state_machine, start_symbol, &lookup);
    let exprted_scans = export_scan_rule_pretty(&scanner_rule, &gramer_rule.symbols, &lookup);

    let output_dir = std::env::current_dir()?.join("src/assets/generated");

    export_to_file(&exported_kinds, &output_dir.join("syntax_kind.rs"))?;
    export_to_file(&export_states, &output_dir.join("parser_state.rs"))?;
    export_to_file(&exprted_scans, &output_dir.join("scan_rule.rs"))?;
    
    Ok(())
}

fn export_to_file<P: AsRef<std::path::Path>>(
    source: &str,
    file_path: &P,
) -> Result<(), anyhow::Error> {
    let file = std::fs::File::create(file_path)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(source.as_bytes())?;
    writer.flush()?;

    Ok(())
}
