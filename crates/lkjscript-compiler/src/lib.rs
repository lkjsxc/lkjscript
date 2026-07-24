//! Compile canonical line-oriented `.lkjscript` source into verified typed SSA
//! and validated reference bytecode.

mod analyze;
mod ast;
mod codegen;
mod effects;
mod hir;
mod import;
mod lex;
mod limits_check;
mod operation;
mod ownership;
mod parse;
mod ssa;
mod types;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lkjscript_core::{validate_chunk, Limits, Result, ValidatedChunk};
use lkjscript_ir::{BytecodeLinkMetadata, VerifiedProgram};

use crate::analyze::{analyze_program, analyze_program_without_effects};
use crate::ast::Expr;
use crate::codegen::compile_program;
use crate::import::{load_program, load_program_with_metrics};
use crate::limits_check::check_file_limits;
use crate::parse::parse_tokens;
use crate::ssa::{lower_program, lower_program_with_metrics};

pub const SOURCE_EXTENSION: &str = "lkjscript";

/// Monotonic direct phase timings for one source compilation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompileMetrics {
    pub total: Duration,
    pub source_loading: Duration,
    pub parsing: Duration,
    pub hir_analysis: Duration,
    pub effect_analysis: Duration,
    pub ssa_construction: Duration,
    pub ssa_verification: Duration,
    pub normalization: Duration,
    pub bytecode_lowering: Duration,
    pub bytecode_validation: Duration,
    pub source_files: usize,
}

/// One compiled semantic program shared by the reference VM and later backends.
#[derive(Debug, Clone)]
pub struct ExecutableProgram {
    bytecode: ValidatedChunk,
    ssa: VerifiedProgram,
    bytecode_links: BytecodeLinkMetadata,
}

impl ExecutableProgram {
    pub fn bytecode(&self) -> &ValidatedChunk {
        &self.bytecode
    }

    pub fn ssa(&self) -> &VerifiedProgram {
        &self.ssa
    }

    pub fn bytecode_links(&self) -> &BytecodeLinkMetadata {
        &self.bytecode_links
    }

    pub fn into_bytecode(self) -> ValidatedChunk {
        self.bytecode
    }
}

pub fn compile_path(path: &Path, limits: &Limits) -> Result<ExecutableProgram> {
    compile_path_with_sources(path, limits).map(|(program, _)| program)
}

/// Compile while retaining low-overhead phase metrics for benchmark tooling.
pub fn compile_path_with_metrics(
    path: &Path,
    limits: &Limits,
) -> Result<(ExecutableProgram, CompileMetrics)> {
    let total_started = Instant::now();
    ensure_source_path(path)?;
    let (program, loading) = load_program_with_metrics(path, limits)?;
    let source_files = program.files.len();

    let hir_started = Instant::now();
    let mut analyzed = analyze_program_without_effects(&program)?;
    let hir_analysis = hir_started.elapsed();
    let effects_started = Instant::now();
    effects::infer(&mut analyzed);
    let effect_analysis = effects_started.elapsed();

    let (ssa, ssa_metrics) = lower_program_with_metrics(&analyzed)?;
    let bytecode_started = Instant::now();
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode_lowering = bytecode_started.elapsed();
    let validation_started = Instant::now();
    let bytecode = validate_chunk(chunk, &limits.validation)?;
    let bytecode_validation = validation_started.elapsed();
    let executable = ExecutableProgram {
        bytecode,
        ssa,
        bytecode_links,
    };
    Ok((
        executable,
        CompileMetrics {
            total: total_started.elapsed(),
            source_loading: loading.source_loading,
            parsing: loading.parsing,
            hir_analysis,
            effect_analysis,
            ssa_construction: ssa_metrics.construction,
            ssa_verification: ssa_metrics.verification,
            normalization: ssa_metrics.normalization,
            bytecode_lowering,
            bytecode_validation,
            source_files,
        },
    ))
}

pub fn compile_path_with_sources(
    path: &Path,
    limits: &Limits,
) -> Result<(ExecutableProgram, Vec<PathBuf>)> {
    ensure_source_path(path)?;
    let program = load_program(path, limits)?;
    let sources = program
        .files
        .iter()
        .map(|source| source.path.clone())
        .collect();
    let analyzed = analyze_program(&program)?;
    let executable = compile_analyzed(&analyzed, limits)?;
    Ok((executable, sources))
}

pub fn compile_source(source: &str, path: &str, limits: &Limits) -> Result<ExecutableProgram> {
    ensure_source_path(Path::new(path))?;
    let forms = parse_source(source, path, limits)?;
    let fake = PathBuf::from(path);
    let program = import::Program {
        root: fake.clone(),
        files: vec![import::SourceFile { path: fake, forms }],
    };
    let analyzed = analyze_program(&program)?;
    compile_analyzed(&analyzed, limits)
}

fn compile_analyzed(analyzed: &hir::Program, limits: &Limits) -> Result<ExecutableProgram> {
    let ssa = lower_program(analyzed)?;
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode = validate_chunk(chunk, &limits.validation)?;
    Ok(ExecutableProgram {
        bytecode,
        ssa,
        bytecode_links,
    })
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
        let integer = "main/\nsig/\n->\nI64\n/sig\n9223372036854775807\n/main\n";
        let integer = compile_source(integer, "integer.lkjscript", &Limits::default())
            .expect("compile I64 source");
        assert!(integer
            .bytecode()
            .constants()
            .iter()
            .any(|constant| matches!(constant, Constant::I64(i64::MAX))));

        let float = "main/\nsig/\n->\nF64\n/sig\n+/\n2.0\n1\n/+\n/main\n";
        let float = compile_source(float, "float.lkjscript", &Limits::default())
            .expect("compile F64 source");
        assert!(float
            .bytecode()
            .constants()
            .iter()
            .any(|constant| matches!(constant, Constant::F64(value) if *value == 2.0)));
    }

    fn ownership_source(body: &str, result: &str) -> String {
        let result = result.replace(' ', "\n");
        format!("main/\nsig/\n->\n{result}\n/sig\n{body}\n/main\n")
    }

    #[test]
    fn initial_owned_buf_slice_accepts_nll_mutation_move_and_return() {
        let source = ownership_source(
            "let/\nbind/\nb\nowned-buf-new/\n2\n/owned-buf-new\n/bind\ndo/\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nowned-buf-len/\nr\n/owned-buf-len\n/let\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\ndo/\nowned-buf-set/\nm\n0\n65\n/owned-buf-set\nmove/\nb\n/move\n/do\n/let\n/do\n/let",
            "Owned Buf",
        );
        let program = compile_source(&source, "owned-valid.lkjscript", &Limits::default())
            .expect("valid Owned Buf safe island");
        assert!(program
            .ssa()
            .program()
            .functions
            .iter()
            .flat_map(|function| &function.blocks)
            .flat_map(|block| &block.instructions)
            .any(|instruction| matches!(
                instruction.kind,
                lkjscript_ir::InstructionKind::Move { .. }
            )));

        let shared_pair = ownership_source(
            "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nlet/\nbind/\nr1\nborrow/\nb\n/borrow\n/bind\nbind/\nr2\nborrow/\nb\n/borrow\n/bind\ndo/\nowned-buf-len/\nr1\n/owned-buf-len\nowned-buf-len/\nr2\n/owned-buf-len\n/do\n/let\n/let",
            "I64",
        );
        compile_source(
            &shared_pair,
            "owned-shared-pair.lkjscript",
            &Limits::default(),
        )
        .expect("overlapping shared loans must be accepted");

        let equal_branch = ownership_source(
            "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nif/\ntrue\nmove/\nb\n/move\nmove/\nb\n/move\n/if\n/let",
            "Owned Buf",
        );
        compile_source(
            &equal_branch,
            "owned-equal-branch.lkjscript",
            &Limits::default(),
        )
        .expect("equal branch move states must join");

        let branch_local_result = ownership_source(
            "if/\ntrue\nlet/\nbind/\na\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nmove/\na\n/move\n/let\nlet/\nbind/\nb\nowned-buf-new/\n2\n/owned-buf-new\n/bind\nmove/\nb\n/move\n/let\n/if",
            "Owned Buf",
        );
        compile_source(
            &branch_local_result,
            "owned-branch-local-result.lkjscript",
            &Limits::default(),
        )
        .expect("transferred branch-local owners must canonicalize at the result join");

        let constant_false_loop = ownership_source("while/\nfalse\nunit\n/while", "Unit");
        compile_source(
            &constant_false_loop,
            "constant-false-loop.lkjscript",
            &Limits::default(),
        )
        .expect("branch simplification must clear a stale loop-header marker");
    }

    #[test]
    fn initial_owned_buf_slice_rejects_affine_and_alias_failures() {
        let cases = [
            ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nb\n/let", "Owned Buf", "loaded or copied"),
            ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nmove/\nb\n/move\nmove/\nb\n/move\n/do\n/let", "Owned Buf", "double move"),
            ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\ndo/\nmove/\nb\n/move\nowned-buf-len/\nr\n/owned-buf-len\n/do\n/let\n/let", "I64", "while it is borrowed"),
            ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nif/\ntrue\ndo/\nmove/\nb\n/move\nunit\n/do\nunit\n/if\n/let", "Unit", "branch join"),
            ("borrow/\nowned-buf-new/\n1\n/owned-buf-new\n/borrow", "Unit", "whole Owned Buf local"),
            ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nmove/\nb\n/move\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/do\n/let", "I64", "after move"),
            ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\ndo/\nowned-buf-len/\nr\n/owned-buf-len\nowned-buf-set/\nm\n0\n1\n/owned-buf-set\n/do\n/let\n/let\n/let", "Unit", "conflicting shared and exclusive"),
            ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nwhile/\nfalse\nmove/\nb\n/move\n/while\n/let", "Unit", "loop-carried"),
            ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nsome/\nborrow/\nb\n/borrow\n/some\nunit\n/do\n/let", "Unit", "cannot be stored in List or Option"),
        ];
        for (body, result, diagnostic) in cases {
            let source = ownership_source(body, result);
            let error = compile_source(&source, "owned-invalid.lkjscript", &Limits::default())
                .expect_err("invalid ownership source must fail")
                .to_string();
            assert!(error.contains(diagnostic), "{diagnostic}: {error}");
        }
    }

    #[test]
    fn ownership_generic_laundering_and_reference_results_are_rejected() {
        let generic_id = "def/\nname/\nid\n/name\nfn/\nforall/\nT\n/forall\nsig/\nT\n->\nT\n/sig\nparams/\nx\nT\n/params\nx\n/fn\n/def\n";
        let reference = format!(
            "{generic_id}{}",
            ownership_source(
                "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nlet/\nbind/\nr2\nid/\nr\n/id\n/bind\ndo/\nmove/\nb\n/move\nowned-buf-len/\nr2\n/owned-buf-len\n/do\n/let\n/let\n/let",
                "I64",
            )
        );
        let owned = format!(
            "{generic_id}{}",
            ownership_source(
                "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nid/\nmove/\nb\n/move\n/id\n/let",
                "Owned Buf",
            )
        );
        let generic_with_owned_parameter = "def/\nname/\nconsume-generic\n/name\nfn/\nforall/\nT\n/forall\nsig/\nOwned\nBuf\nT\n->\nT\n/sig\nparams/\nb\nOwned/\nBuf\n/Owned\nx\nT\n/params\nx\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nconsume-generic/\nmove/\nb\n/move\n7\n/consume-generic\n/let\n/main\n";
        for source in [reference, owned, generic_with_owned_parameter.into()] {
            let error = compile_source(&source, "generic-owned.lkjscript", &Limits::default())
                .expect_err("generic ownership laundering must fail")
                .to_string();
            assert!(
                error.contains("ownership/reference generic instantiation is unavailable"),
                "wrong generic ownership diagnostic: {error}"
            );
        }
    }

    #[test]
    fn ownership_function_signature_escape_boundary_is_exact() {
        let valid = "def/\nname/\nread-owned\n/name\nfn/\nsig/\nRef\nBuf\n->\nI64\n/sig\nparams/\nr\nRef/\nBuf\n/Ref\n/params\nowned-buf-len/\nr\n/owned-buf-len\n/fn\n/def\ndef/\nname/\nwrite-owned\n/name\nfn/\nsig/\nRefMut\nBuf\n->\nUnit\n/sig\nparams/\nr\nRefMut/\nBuf\n/RefMut\n/params\nowned-buf-set/\nr\n0\n1\n/owned-buf-set\n/fn\n/def\ndef/\nname/\nfresh-owned\n/name\nfn/\nsig/\nI64\n->\nOwned\nBuf\n/sig\nparams/\nn\nI64\n/params\nowned-buf-new/\nn\n/owned-buf-new\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nfresh-owned/\n1\n/fresh-owned\n/bind\nread-owned/\nborrow/\nb\n/borrow\n/read-owned\n/let\n/main\n";
        compile_source(valid, "ownership-signatures.lkjscript", &Limits::default())
            .expect("Ref/RefMut parameters and Owned return must remain valid");

        let consumed_ref_mut_before_safepoint = "def/\nname/\nwrite-then-allocate\n/name\nfn/\nsig/\nRefMut\nBuf\n->\nI64\n/sig\nparams/\nr\nRefMut/\nBuf\n/RefMut\n/params\ndo/\nowned-buf-set/\nr\n0\n1\n/owned-buf-set\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/let\n/do\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
        compile_source(
            consumed_ref_mut_before_safepoint,
            "consumed-ref-mut-frame.lkjscript",
            &Limits::default(),
        )
        .expect("consumed RefMut must leave later semantic frame states");

        let invalid = "def/\nname/\nreturn-ref\n/name\nfn/\nsig/\nRef\nBuf\n->\nRef\nBuf\n/sig\nparams/\nr\nRef/\nBuf\n/Ref\n/params\nr\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
        let error = compile_source(invalid, "reference-return.lkjscript", &Limits::default())
            .expect_err("reference return must fail")
            .to_string();
        assert!(error.contains("cannot be returned"), "{error}");
    }

    #[test]
    fn ownership_types_cannot_escape_into_products_or_collections() {
        let product_direct = "product/\nname/\nBad\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nOwned\nBuf\n/type\n/field\n/fields\n/product\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
        let product_nested = "product/\nname/\nBadNested\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nOption\nRef\nBuf\n/type\n/field\n/fields\n/product\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
        let list = "main/\nsig/\n->\nList\nOwned\nBuf\n/sig\nunit\n/main\n";
        let option = "main/\nsig/\n->\nOption\nRef\nBuf\n/sig\nunit\n/main\n";
        let result = "main/\nsig/\n->\nResult\nI64\nRefMut\nBuf\n/sig\nunit\n/main\n";
        for source in [product_direct, product_nested, list, option, result] {
            let error = compile_source(source, "stored-owned.lkjscript", &Limits::default())
                .expect_err("ownership storage must fail")
                .to_string();
            assert!(
                error.contains("ownership/reference"),
                "wrong ownership storage diagnostic: {error}"
            );
        }
    }

    #[test]
    fn lexical_owned_places_join_without_branch_local_pollution() {
        let valid_local = ownership_source(
            "if/\ntrue\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/let\nlet/\nbind/\nb\nowned-buf-new/\n2\n/owned-buf-new\n/bind\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/let\n/if",
            "I64",
        );
        compile_source(
            &valid_local,
            "branch-local-owned.lkjscript",
            &Limits::default(),
        )
        .expect("branch-local Owned places must end before the join");

        let valid_reinit = ownership_source(
            "var/\nname/\nb\n/name\ntype/\nOwned\nBuf\n/type\nowned-buf-new/\n1\n/owned-buf-new\ndo/\nif/\ntrue\nmove/\nb\n/move\nmove/\nb\n/move\n/if\nset/\nb\nowned-buf-new/\n3\n/owned-buf-new\n/set\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/do\n/var",
            "I64",
        );
        compile_source(
            &valid_reinit,
            "branch-reinit-owned.lkjscript",
            &Limits::default(),
        )
        .expect("equal branch moves may be reinitialized after the join");

        let invalid_after_move = ownership_source(
            "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nmove/\nb\n/move\nif/\ntrue\nlet/\nbind/\nlocal\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nunit\n/let\nunit\n/if\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/do\n/let",
            "I64",
        );
        let error = compile_source(
            &invalid_after_move,
            "branch-after-move.lkjscript",
            &Limits::default(),
        )
        .expect_err("branch-local place must not resurrect a moved outer place")
        .to_string();
        assert!(
            error.contains("after move"),
            "wrong move diagnostic: {error}"
        );

        let invalid_reinit = ownership_source(
            "var/\nname/\nb\n/name\ntype/\nOwned\nBuf\n/type\nowned-buf-new/\n1\n/owned-buf-new\nset/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/set\n/var",
            "Unit",
        );
        let error = compile_source(
            &invalid_reinit,
            "owned-reinit-before-move.lkjscript",
            &Limits::default(),
        )
        .expect_err("initialized Owned var cannot be overwritten")
        .to_string();
        assert!(
            error.contains("only reinitialization after move"),
            "{error}"
        );
    }

    #[test]
    fn temporary_borrows_have_only_direct_supported_placements() {
        let direct = ownership_source(
            "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\nmove/\nb\n/move\n/do\n/let",
            "Owned Buf",
        );
        compile_source(&direct, "temporary-borrow.lkjscript", &Limits::default())
            .expect("direct temporary borrow must end after the operation");

        let unsupported = [
            "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nborrow/\nb\n/borrow\nunit\n/do\n/let",
            "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-len/\nif/\ntrue\nborrow/\nb\n/borrow\nborrow/\nb\n/borrow\n/if\n/owned-buf-len\n/let",
            "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-len/\ndo/\nborrow/\nb\n/borrow\n/do\n/owned-buf-len\n/let",
        ];
        for body in unsupported {
            let source = ownership_source(body, if body.contains("unit") { "Unit" } else { "I64" });
            let error = compile_source(
                &source,
                "unsupported-borrow-placement.lkjscript",
                &Limits::default(),
            )
            .expect_err("unsupported Borrow placement must fail")
            .to_string();
            assert!(
                error.contains("exact direct reference argument or direct let initializer"),
                "wrong Borrow placement diagnostic: {error}"
            );
        }

        let borrow_then_move_call = "def/\nname/\nobserve-and-take\n/name\nfn/\nsig/\nRef\nBuf\nOwned\nBuf\n->\nI64\n/sig\nparams/\nr\nRef/\nBuf\n/Ref\nb\nOwned/\nBuf\n/Owned\n/params\nowned-buf-len/\nr\n/owned-buf-len\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nobserve-and-take/\nborrow/\nb\n/borrow\nmove/\nb\n/move\n/observe-and-take\n/let\n/main\n";
        let error = compile_source(
            borrow_then_move_call,
            "temporary-full-call.lkjscript",
            &Limits::default(),
        )
        .expect_err("temporary loan must cover all call arguments")
        .to_string();
        assert!(error.contains("while it is borrowed"), "{error}");
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
