use wit_bindgen::generate;

generate!({
    world: "app-world",
    path: "src/bin/parser_app/assets/wit",
    default_bindings_module: "crate::api::bindings::app_world",
    generate_all,
});
