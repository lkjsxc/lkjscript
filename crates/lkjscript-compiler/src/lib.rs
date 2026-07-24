//! Compile canonical line-oriented `.lkjscript` source into bytecode.

mod analyze;
mod ast;
mod codegen;
mod hir;
mod import;
mod lex;
mod limits_check;
mod operation;
mod parse;
mod types;

use std::path::{Path, PathBuf};

use lkjscript_core::{Chunk, Limits, Result};

use crate::analyze::analyze_program;
use crate::ast::Expr;
use crate::codegen::compile_program;
use crate::import::load_program;
use crate::limits_check::check_file_limits;
use crate::parse::parse_tokens;

pub const SOURCE_EXTENSION: &str = "lkjscript";

pub fn compile_path(path: &Path, limits: &Limits) -> Result<Chunk> {
    compile_path_with_sources(path, limits).map(|(chunk, _)| chunk)
}

pub fn compile_path_with_sources(path: &Path, limits: &Limits) -> Result<(Chunk, Vec<PathBuf>)> {
    ensure_source_path(path)?;
    let program = load_program(path, limits)?;
    let sources = program
        .files
        .iter()
        .map(|source| source.path.clone())
        .collect();
    let analyzed = analyze_program(&program)?;
    let chunk = compile_program(&analyzed)?;
    Ok((chunk, sources))
}

pub fn compile_source(source: &str, path: &str, limits: &Limits) -> Result<Chunk> {
    ensure_source_path(Path::new(path))?;
    let forms = parse_source(source, path, limits)?;
    let fake = PathBuf::from(path);
    let program = import::Program {
        root: fake.clone(),
        files: vec![import::SourceFile { path: fake, forms }],
    };
    let analyzed = analyze_program(&program)?;
    compile_program(&analyzed)
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
    let tokens =
        lex::lex(source).map_err(|error| lkjscript_core::Error::msg(format!("{path}: {error}")))?;
    check_file_limits(&tokens, limits, path)?;
    let forms = parse_tokens(&tokens)
        .map_err(|error| lkjscript_core::Error::msg(format!("{path}: {error}")))?;
    import::validate_top_level(&forms, limits, path)?;
    Ok(forms)
}

pub use lkjscript_core::Limits as CompileLimits;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::Path;

    use lkjscript_core::{Constant, Limits};

    use super::{compile_source, ensure_source_path};

    fn unit_main(body: &str) -> String {
        format!("main/\nsig/\n->\nUnit\n/sig\ndo/\n{body}\nunit\n/do\n/main\n")
    }

    #[test]
    fn accepts_only_canonical_source_extension() {
        assert!(ensure_source_path(Path::new("main.lkjscript")).is_ok());
        assert!(ensure_source_path(Path::new("main.lkjml")).is_err());
        assert!(ensure_source_path(Path::new("main")).is_err());
    }

    #[test]
    fn bounded_terminal_operations_replace_arbitrary_ioctl() {
        let canonical = unit_main(
            "sys-tty-get/\nstdin-handle/\n/stdin-handle\nbuf-new/\n60\n/buf-new\n/sys-tty-get",
        );
        let arbitrary = unit_main(
            "sys-ioctl/\nstdin-handle/\n/stdin-handle\n21505\nbuf-new/\n1\n/buf-new\n/sys-ioctl",
        );
        assert!(compile_source(&canonical, "terminal.lkjscript", &Limits::default()).is_ok());
        assert!(compile_source(&arbitrary, "terminal.lkjscript", &Limits::default()).is_err());
    }

    #[test]
    fn descriptor_names_are_handle_and_result_explicit() {
        let canonical =
            unit_main("is-ok/\nsys-close/\nstdin-handle/\n/stdin-handle\n/sys-close\n/is-ok");
        let obsolete = unit_main("close/\nstdin-handle/\n/stdin-handle\n/close");
        assert!(compile_source(&canonical, "handles.lkjscript", &Limits::default()).is_ok());
        assert!(compile_source(&obsolete, "handles.lkjscript", &Limits::default()).is_err());
    }

    #[test]
    fn bytecode_constants_preserve_numeric_source_types() {
        let source = unit_main(
            "equal-value/\n9223372036854775807\n9223372036854775807\n/equal-value\n+/\n2.0\n1\n/+",
        );
        let chunk = compile_source(&source, "numeric.lkjscript", &Limits::default())
            .expect("compile numeric source");
        assert!(chunk
            .constants
            .iter()
            .any(|constant| matches!(constant, Constant::I64(i64::MAX))));
        assert!(chunk
            .constants
            .iter()
            .any(|constant| matches!(constant, Constant::F64(value) if *value == 2.0)));
    }

    #[test]
    fn removed_numeric_vocabulary_and_non_binary_arithmetic_fail() {
        for ty in [
            "I32", "U32", "U64", "F32", "I128", "U8", "F16", "i32", "i64", "u32", "u64", "f32",
            "f64", "i128", "Int", "Float",
        ] {
            let source = format!("main/\nsig/\n->\n{ty}\n/sig\n1\n/main\n");
            let error = compile_source(&source, "removed-type.lkjscript", &Limits::default())
                .expect_err("removed numeric type must fail")
                .to_string();
            assert!(
                error.contains("unsupported numeric type"),
                "wrong diagnostic for {ty}: {error}"
            );
        }
        for name in [
            "eq",
            "ne",
            "f+",
            "f-",
            "f*",
            "f=",
            "f!=",
            "f<",
            "f<=",
            "f>",
            "f>=",
            "le",
            "ge",
            "=",
            "!=",
            "<",
            "<=",
            ">",
            ">=",
            "i64-from-u32",
            "u32-from-i64",
            "i64-from-i32",
            "i32-from-i64",
        ] {
            let source = unit_main(&format!("{name}/\n1\n2\n/{name}"));
            assert!(
                compile_source(&source, "removed-op.lkjscript", &Limits::default()).is_err(),
                "accepted operator {name}"
            );
        }
        let variadic = unit_main("+/\n1\n2\n3\n/+");
        assert!(compile_source(&variadic, "arity.lkjscript", &Limits::default()).is_err());
    }
}
