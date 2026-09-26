use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let directory = root.join("decoders");
    println!("cargo:rerun-if-changed=decoders");
    let mut files: Vec<_> = fs::read_dir(&directory)
        .expect("decoders directory")
        .map(|entry| entry.expect("decoder entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    files.sort();
    let mut code = String::new();
    let mut names = Vec::new();
    for path in files {
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .expect("UTF-8 filename");
        let (manufacturer, slug) = name.split_once('_').expect("use <code>_<slug>.rs");
        assert!(
            manufacturer.len() == 3
                && manufacturer.bytes().all(|b| b.is_ascii_lowercase())
                && !slug.is_empty()
                && slug
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "use three lowercase manufacturer letters and a lowercase snake_case slug"
        );
        code.push_str(&format!(
            "pub mod {name} {{ include!({path:?}); pub static DECODER: Decoder = Decoder;\n\
             #[cfg(test)] #[test] fn known_answers() {{\n\
             assert_eq!(crate::ManufacturerDecoder::descriptor(&DECODER).selector.manufacturer, *b\"{}\");\n\
             crate::testing::assert_fixtures(&DECODER, tests::FIXTURES);\n\
             }} }}\npub use {name}::DECODER as {};\n",
            manufacturer.to_ascii_uppercase(),
            name.to_ascii_uppercase(),
        ));
        names.push(name.to_owned());
    }
    code.push_str("pub static BUILTIN: &[&dyn crate::ManufacturerDecoder] = &[\n");
    for name in names {
        code.push_str(&format!("&{name}::DECODER,\n"));
    }
    code.push_str("];");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").expect("output directory")).join("builtins.rs"),
        code,
    )
    .expect("write generated registry");
}
