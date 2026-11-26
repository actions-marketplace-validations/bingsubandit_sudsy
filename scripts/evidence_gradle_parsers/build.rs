// Compiles the tree-sitter grammar named by GRAMMAR_SRC (a `src` folder with parser.c and optionally scanner.c or scanner.cc);
// GRAMMAR_SYMBOL is the exported `tree_sitter_<name>` function (default tree_sitter_groovy).
use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-env-changed=GRAMMAR_SRC");
    println!("cargo:rerun-if-env-changed=GRAMMAR_SYMBOL");
    let out = env::var("OUT_DIR").unwrap();
    let sym = env::var("GRAMMAR_SYMBOL").unwrap_or_else(|_| "tree_sitter_groovy".into());
    let body = if env::var("GRAMMAR_SRC").is_ok() {
        format!("extern \"C\" {{ fn {sym}() -> *const (); }}\npub fn lang() -> tree_sitter::Language {{ tree_sitter::Language::from(unsafe {{ tree_sitter_language::LanguageFn::from_raw({sym}) }}) }}\n")
    } else {
        "pub fn lang() -> tree_sitter::Language { panic!(\"built with no grammar\") }\n".to_string()
    };
    fs::write(Path::new(&out).join("symbol.rs"), body).unwrap();
    let Ok(src) = env::var("GRAMMAR_SRC") else { return };
    let src = Path::new(&src);
    let mut c = cc::Build::new();
    c.include(src).opt_level(2).warnings(false).file(src.join("parser.c"));
    if src.join("scanner.c").exists() {
        c.file(src.join("scanner.c"));
    }
    c.compile("grammar");
    if src.join("scanner.cc").exists() {
        cc::Build::new().cpp(true).include(src).opt_level(2).warnings(false).file(src.join("scanner.cc")).compile("grammar_scanner");
    }
}
