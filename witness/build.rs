//! Adapt lkjscript module filenames for the optional Rust witness only.
//! Source bytes and embedded .lkj inputs remain unchanged.
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
                fs::copy(&input, output.with_extension("rs"))?;
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
    for (file, module) in [("lib.rs", "language"), ("embedded.rs", "embedded")] {
        let path = source.join("src").join(file);
        fs::write(
            output.join(format!("{module}-module.rs")),
            format!("#[path = {path:?}]\nmod {module};\n"),
        )?;
    }
    Ok(())
}
