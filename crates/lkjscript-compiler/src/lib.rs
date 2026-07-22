//! Compile canonical line-oriented `.lkjscript` source into bytecode.

mod ast;
mod codegen;
mod import;
mod lex;
mod limits_check;
mod parse;
mod types;

use std::path::{Path, PathBuf};

use lkjscript_core::{Chunk, Limits, Result};

use crate::ast::Expr;
use crate::codegen::compile_program;
use crate::import::load_program;
use crate::limits_check::check_file_limits;
use crate::parse::parse_tokens;

pub const SOURCE_EXTENSION: &str = "lkjscript";

pub fn compile_path(path: &Path, limits: &Limits) -> Result<Chunk> {
    ensure_source_path(path)?;
    let program = load_program(path, limits)?;
    compile_program(&program)
}

pub fn compile_source(source: &str, path: &str, limits: &Limits) -> Result<Chunk> {
    ensure_source_path(Path::new(path))?;
    let forms = parse_source(source, path, limits)?;
    let fake = PathBuf::from(path);
    let program = import::Program {
        root: fake.clone(),
        files: vec![import::SourceFile {
            path: fake,
            forms,
        }],
    };
    compile_program(&program)
}

pub fn validate_source(source: &str, path: &str, limits: &Limits) -> Result<()> {
    ensure_source_path(Path::new(path))?;
    parse_source(source, path, limits).map(|_| ())
}

pub fn validate_source_tree(root: &Path, limits: &Limits) -> Result<()> {
    import::validate_source_tree(root, limits)
}

pub(crate) fn ensure_source_path(path: &Path) -> Result<()> {
    if path.extension().and_then(|extension| extension.to_str()) == Some(SOURCE_EXTENSION) {
        return Ok(());
    }
    Err(lkjscript_core::Error::msg(format!(
        "source path must end in .{SOURCE_EXTENSION}: {}",
        path.display()
    )))
}

fn parse_source(source: &str, path: &str, limits: &Limits) -> Result<Vec<Expr>> {
    let tokens = lex::lex(source)
        .map_err(|error| lkjscript_core::Error::msg(format!("{path}: {error}")))?;
    check_file_limits(&tokens, limits, path)?;
    let forms = parse_tokens(&tokens)
        .map_err(|error| lkjscript_core::Error::msg(format!("{path}: {error}")))?;
    import::validate_top_level(&forms, limits, path)?;
    Ok(forms)
}

pub use lkjscript_core::Limits as CompileLimits;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::ensure_source_path;

    #[test]
    fn accepts_only_canonical_source_extension() {
        assert!(ensure_source_path(Path::new("main.lkjscript")).is_ok());
        assert!(ensure_source_path(Path::new("main.lkjml")).is_err());
        assert!(ensure_source_path(Path::new("main")).is_err());
    }
}
