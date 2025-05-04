use std::process::Command;

#[allow(dead_code)]
fn main() {
    if let Err(err) = run_main() {
        eprintln!("Build failed: {:?}", err);
        panic!();
    }
}

fn run_main() -> Result<(), anyhow::Error> {
    let artifact_dir = std::env::var("BUILD_DIR").unwrap_or_else(|_| "build".to_string());
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let bin_name = std::env::var("CARGO_FEATURE_KEYWORD_MATCHER").unwrap_or_default();
    eprintln!("**** CARGO_BIN_NAME: {}", bin_name);

    if std::env::var("CARGO_FEATURE_KEYWORD_MATCHER").is_ok() {
        let gen_command = format!("{out_dir}/mkkeywordhash");
        let gen_c_source = format!("{artifact_dir}/keywordhash.h");
        run_build_c("submodules/sqlite/tool/mkkeywordhash.c", &gen_command)?;
        run_command_generated(&gen_command, Some(&gen_c_source))?;
    }

    if std::env::var("CARGO_FEATURE_GRAMMAR_CONVERTER").is_ok() {
        let c_source_headers = [format!("{artifact_dir}/keywords.h"), format!("{artifact_dir}/keywordhash.h")];
        let c_source = format!("{artifact_dir}/keyword_check.c");
        run_merge_c_headers(&c_source_headers, &c_source)?;
        // println!("cargo:rerun-if-changed={}", format!("{artifact_dir}/keywords.h"));

        run_bindgen("src/assets/sqlite/keyword_check.h", "src/binding/keyword_check.rs")?;

        run_build_binding_lib(&c_source, "keyword_check");

        println!("cargo:rustc-link-lib=static=lemon");
        println!("cargo:rustc-link-lib=static=keyword_check");
            // println!("cargo:rerun-if-changed={artifact_dir}/keywordhash.h", );
    }

    let lemon_c = "../submodules/sqlite/tool/lemon.c";
    run_build_binding_lib(lemon_c, "lemon");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=lemon");

    Ok(())
}

fn run_bindgen(header_path: &str, out_path: &str) -> Result<(), anyhow::Error> {
    let bindings = bindgen::Builder::default()
        .header(header_path) 
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .map_err(|_| anyhow::anyhow!("Unable to generate bindings: `{header_path}`"))?
    ;

    bindings.write_to_file(std::path::Path::new(out_path))?;

    println!("cargo:rerun-if-changed={}", header_path);

    Ok(())
}

fn run_build_c(source_path: &str, out_path: &str) -> Result<(), anyhow::Error> {
    eprintln!("run_build_c called !");
    println!("cargo:rerun-if-changed={source_path}");

    let status = Command::new("clang")
        .arg(source_path)
        .args(&["-o", out_path])
        .status()
        .map_err(|_| anyhow::anyhow!("Failed to compile {source_path}"))?
    ;
    assert!(status.success(), "Clang compilation failed");

    Ok(())
}

fn run_command_generated(command_path: &str, redirect_path: Option<&String>) -> Result<(), anyhow::Error> {
    let mut command = Command::new(&command_path);

    if let Some(redirect_path) = redirect_path {
        let file = std::fs::File::create(redirect_path).map_err(|_| anyhow::anyhow!("Failed to create {redirect_path}"))?;
        command.stdout(file);
    }

    let mut child = command.spawn().map_err(|_| anyhow::anyhow!("Failed to execute {command_path}"))?;
    let status = child.wait().map_err(|_| anyhow::anyhow!("Failed to wait for gen_code"))?;

    assert!(status.success(), "{command_path} execution failed");

    Ok(())
}

fn run_merge_c_headers(sources: &[String], out_path: &str) -> Result<(), anyhow::Error> {
    use std::io::BufRead;
    use std::io::Write;

    let out_file = std::fs::File::create(out_path).map_err(|_| anyhow::anyhow!("Can not create file: `{out_path}`"))?;
    let mut writer = std::io::BufWriter::new(out_file);

    for source in sources {
        let in_file = std::fs::File::open(source).map_err(|_| anyhow::anyhow!("Can not open merge source: `{source}`"))?;
        let reader = std::io::BufReader::new(in_file);
        for line in reader.lines() {
            let line = line?; 
            writeln!(writer, "{}", line)?; 
        }
    }

    Ok(())
}

fn run_build_binding_lib(c_source: &str, lib_name: &str) {
    cc::Build::new()
        .file(c_source) 
        .define("main", "dont_use_main")
        .warnings(true)
        .compile(lib_name)
    ;

    println!("cargo:rerun-if-changed={c_source}");
}