#[allow(dead_code)]
fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let lemon_c = "submodules/sqlite/tool/lemon.c";

    cc::Build::new()
        .file(lemon_c) 
        .define("main", "dont_use_main")
        .warnings(true)
        .compile("lemon")
    ;

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=lemon");
    println!("cargo:rerun-if-changed={lemon_c}");
}