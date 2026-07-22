//! Compile line-oriented `.lkjml` source into bytecode.

mod ast;
mod codegen;
mod import;
mod lex;
mod limits_check;
mod parse;
mod types;

use std::path::{Path, PathBuf};

use lkjscript2026_core::{Chunk, Limits, Result};

use crate::ast::Expr;
use crate::codegen::compile_program;
use crate::import::load_program;
use crate::limits_check::check_file_limits;
use crate::parse::parse_tokens;

pub fn compile_path(path: &Path, limits: &Limits) -> Result<Chunk> {
    let program = load_program(path, limits)?;
    compile_program(&program)
}

pub fn compile_source(source: &str, path: &str, limits: &Limits) -> Result<Chunk> {
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
    parse_source(source, path, limits).map(|_| ())
}

fn parse_source(source: &str, path: &str, limits: &Limits) -> Result<Vec<Expr>> {
    let tokens = lex::lex(source)
        .map_err(|error| lkjscript2026_core::Error::msg(format!("{path}: {error}")))?;
    check_file_limits(&tokens, limits, path)?;
    let forms = parse_tokens(&tokens)
        .map_err(|error| lkjscript2026_core::Error::msg(format!("{path}: {error}")))?;
    import::validate_top_level(&forms, limits, path)?;
    Ok(forms)
}

pub use lkjscript2026_core::Limits as CompileLimits;
