use wit_bindgen::generate;

generate!({
    world: "app-world",
    path: "src/bin/scanner_app/assets/wit",
    default_bindings_module: "crate::api::bindings",
    generate_all,
});
