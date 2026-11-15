//! Adapt lkjscript module filenames for the optional Rust witness only.
//! Original .lkj bytes and embedded inputs remain unchanged. Rust module copies
//! bind the language's JSON/ASCII helpers and capability prelude explicitly.
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn rust_bindings(input: &Path, text: String) -> String {
    let structural = matches!(input.file_name().and_then(|name| name.to_str()),
        Some("structure_edit.lkj" | "structure_write.lkj" | "structure_refactor.lkj" | "structure_replay.lkj"));
    let mut out = String::new();
    for line in text.split_inclusive('\n') {
        if structural && line.trim_start().starts_with("use std::json::") {
            let rebound = line.replacen("use std::json::", "use crate::source_json::", 1);
            out.push_str(&rebound.replace("{self,", "{self as json,"));
        } else { out.push_str(line); }
    }
    if input.ends_with("src/lib.lkj") {
        out = out.replace("pub use std::sha256;", "#[path = \"../library/sha256.lkj\"]\npub mod sha256;");
    }
    if input.ends_with("src/structure_write.lkj") {
        out = out.replace("use std::collections::BTreeSet;",
            "use std::collections::BTreeSet;\nuse crate::host::{Directory, Entropy, File, World};");
    }
    if input.ends_with("src/driver.lkj") {
        out = out.replace("\nmod structure;\n", "\n#[path = \"driver/structure.rs\"]\nmod structure;\n");
    }
    if input.ends_with("src/driver/structure.lkj") {
        out = out.replace("use super::{Options, Stats, Memo};",
            "use super::{Options, Stats, Memo};\nuse crate::host::World;");
        out = out.replace("use super::Options;", "use super::Options;\n    use crate::host::World;");
    }
    if structural || input.ends_with("src/semantic_facts.lkj") {
        out = out.replace("std::str::trim_ascii(", "crate::trim_source_ascii(");
    }
    out
}

fn mirror(source: &Path, destination: &Path, rust_modules: bool) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let input = entry.path();
        let output = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            mirror(&input, &output, rust_modules)?;
        } else {
            fs::copy(&input, &output)?;
            if rust_modules && input.extension().is_some_and(|extension| extension == "lkj") {
                fs::write(output.with_extension("rs"), rust_bindings(&input, fs::read_to_string(&input)?))?;
            }
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("..");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let source = output.join("source");
    if source.exists() { fs::remove_dir_all(&source)?; }
    for name in ["src", "library"] {
        println!("cargo:rerun-if-changed=../{name}");
        mirror(&root.join(name), &source.join(name), name == "src")?;
    }
    for (file, module) in [("lib.rs", "language"), ("embedded.rs", "embedded"),
        ("meaning.rs", "meaning"), ("semantic_store.rs", "semantic_store"), ("semantic_query.rs", "semantic_query")] {
        let path = source.join("src").join(file);
        fs::write(
            output.join(format!("{module}-module.rs")),
            format!("#[path = {path:?}]\nmod {module};\n"),
        )?;
    }
    let json = fs::read_to_string(root.join("library/json.lkj"))?
        .replace("use crate::{BTreeSet, CheckedArithmetic, Option, Result, String};", "use std::collections::BTreeSet;")
        .replace("use crate::option::", "use std::option::")
        .replace("use crate::result::", "use std::result::")
        .replace("crate::fmt::", "std::fmt::");
    let json_path = source.join("library/json.rs");
    fs::write(&json_path, json)?;
    fs::write(output.join("source-json-module.rs"),
        format!("#[path = {json_path:?}]\npub mod source_json;\n"))?;
    Ok(())
}
