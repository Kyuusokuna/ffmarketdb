#[derive(Debug)]
struct CustomParseCallback;

impl bindgen::callbacks::ParseCallbacks for CustomParseCallback {
    fn add_derives(&self, _name: &str) -> Vec<String> {
        return vec!["serde::Serialize".to_string()];
    }
}

fn main() {
    cc::Build::new()
        .file("vendor/ListingsDB.c")
        .compile("listingsdb");

    bindgen::Builder::default()
        .allowlist_file("vendor/ListingsDB.h")
        .header("vendor/ListingsDB.h")
        .parse_callbacks(Box::new(CustomParseCallback))
        .generate()
        .expect("Failed to generate bindings for listingsdb.")
        .write_to_file("src/bindings.rs")
        .expect("Failed to write listingsdb bindings.");
}