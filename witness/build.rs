//! Adapt lkjscript module filenames for the optional Rust witness only.
//! Original .lkj bytes and embedded inputs remain unchanged. Rust module copies
//! bind the language's JSON/ASCII helpers and capability prelude explicitly.
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// Rebind one source-library helper while preserving embedded source and comments.
// Rust's std::io has no capability Directory; the witness bridge supplies it.
fn bind_directory_identity(text: &str) -> String {
    let bytes = text.as_bytes();
    let needle = b"std::io::directory_identity";
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        let start = at;
        if bytes[at..].starts_with(b"//") {
            at += 2;
            while at < bytes.len() && bytes[at] != b'\n' { at += 1; }
        } else if bytes[at..].starts_with(b"/*") {
            at += 2;
            let mut depth = 1usize;
            while at < bytes.len() && depth != 0 {
                if bytes[at..].starts_with(b"/*") { depth += 1; at += 2; }
                else if bytes[at..].starts_with(b"*/") { depth -= 1; at += 2; }
                else { at += 1; }
            }
        } else if bytes[at] == b'"' {
            at += 1;
            while at < bytes.len() {
                let byte = bytes[at]; at += 1;
                if byte == b'\\' && at < bytes.len() { at += 1; }
                else if byte == b'"' { break; }
            }
        } else {
            let mut raw = at + 1;
            if bytes[at] == b'r' {
                while raw < bytes.len() && bytes[raw] == b'#' { raw += 1; }
            }
            if bytes[at] == b'r' && raw < bytes.len() && bytes[raw] == b'"' {
                let hashes = raw - at - 1;
                at = raw + 1;
                while at < bytes.len() {
                    if bytes[at] == b'"' && at + 1 + hashes <= bytes.len()
                        && bytes[at + 1..at + 1 + hashes].iter().all(|byte| *byte == b'#') {
                        at += 1 + hashes; break;
                    }
                    at += 1;
                }
            } else if bytes[at] == b'\'' && at + 1 < bytes.len() {
                let mut end = at + 1;
                if bytes[end] == b'\\' {
                    while end < bytes.len() {
                        if bytes[end] == b'\\' { end += 2; }
                        else if bytes[end] == b'\'' { break; }
                        else { end += 1; }
                    }
                } else {
                    end += text[end..].chars().next().unwrap().len_utf8();
                }
                if end < bytes.len() && bytes[end] == b'\'' { at = end + 1; }
                else { at += 1; } // Lifetime, not a character literal.
            } else if bytes[at..].starts_with(needle)
                && (at == 0 || !(bytes[at - 1].is_ascii_alphanumeric()
                    || matches!(bytes[at - 1], b'_' | b':' | b'#')))
                && (at + needle.len() == bytes.len()
                    || !(bytes[at + needle.len()].is_ascii_alphanumeric() || bytes[at + needle.len()] == b'_')) {
                out.extend_from_slice(b"crate::host::directory_identity");
                at += needle.len();
                continue;
            } else {
                at += 1;
            }
        }
        out.extend_from_slice(&bytes[start..at]);
    }
    String::from_utf8(out).unwrap()
}

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
    if input.ends_with("src/store.lkj") {
        out = out.replace("reuse::ProofDomain::new(", "crate::proof_domain(");
    }
    if input.ends_with("src/structure_write.lkj") {
        out = out.replace("use std::collections::BTreeSet;",
            "use std::collections::BTreeSet;\nuse crate::host::{Directory, Entropy, File, World};");
    }
    if input.ends_with("src/native/units.lkj") {
        out = out.replacen("use super::*;", "use super::*;\nuse crate::host::File;", 1);
    }
    if input.ends_with("src/driver.lkj") {
        for module in ["structure", "units", "help"] {
            out = out.replace(&format!("\nmod {module};\n"),
                &format!("\n#[path = \"driver/{module}.rs\"]\nmod {module};\n"));
        }
    }
    if input.ends_with("src/driver/units.lkj") {
        out = out.replace("\nmod inputs;\n", "\n#[path = \"units/inputs.rs\"]\nmod inputs;\n");
    }
    if input.ends_with("src/driver/structure.lkj") {
        out = out.replace("use super::{Options, Stats, Memo};",
            "use super::{Options, Stats, Memo};\nuse crate::host::World;");
        out = out.replace("use super::Options;", "use super::Options;\n    use crate::host::World;");
    }
    if structural || input.ends_with("src/semantic_facts.lkj") {
        out = out.replace("std::str::trim_ascii(", "crate::trim_source_ascii(");
    }
    bind_directory_identity(&out)
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
        ("meaning.rs", "meaning"), ("semantic_store.rs", "semantic_store"), ("semantic_query.rs", "semantic_query"), ("units.rs", "units"),
        ("residual.rs", "residual")] {
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

#[test]
fn directory_binding_preserves_source_literals_and_other_names() {
    let text = concat!(
        "fn call<'a>(root: &'a mut Directory) { std::io::directory_identity(root); }\n",
        "let quoted = \"std::io::directory_identity(root)\";\n",
        "let raw = r###\"std::io::directory_identity(root)\"###;\n",
        "let byte_raw = br#\"std::io::directory_identity(root)\"#;\n",
        "let quote = '\\\"'; let unicode = 'λ'; let apostrophe = '\\\'';\n",
        "// std::io::directory_identity(root)\n",
        "/* outer /* std::io::directory_identity(root) */ tail */\n",
        "other::std::io::directory_identity(root); std::io::directory_identity_extra(root);\n",
        "fn again() { std::io::directory_identity(root); }\n",
    );
    let expected = text
        .replace("{ std::io::directory_identity(root); }", "{ crate::host::directory_identity(root); }");
    assert_eq!(bind_directory_identity(text), expected);
}
