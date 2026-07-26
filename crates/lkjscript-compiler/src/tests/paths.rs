use std::path::Path;

use super::*;
use crate::{ensure_source_path, types::Type, validate_source};

fn filesystem_main(path_expression: &str) -> String {
    format!(
        concat!(
            "main/\nsig/\nCapability/\nFileSystem\n/Capability\n->\nUnit\n/sig\n",
            "params/\nfile-system\nCapability/\nFileSystem\n/Capability\n/params\n",
            "let/\nbind/\nhandle\nunwrap-ok/\nsys-open-read/\nfile-system\n",
            "{path_expression}\n/sys-open-read\n/unwrap-ok\n/bind\n",
            "do/\nunwrap-ok/\ndrop/\nhandle\n/drop\n/unwrap-ok\nunit\n/do\n/let\n/main\n"
        ),
        path_expression = path_expression,
    )
}

#[test]
fn filesystem_operations_require_path_instead_of_str() {
    let source = filesystem_main("str/\n/tmp/rejected-string-path\n/str");
    let error = compile_source(&source, "string-path.lkjscript", &Limits::default())
        .expect_err("Str pathname must fail")
        .to_string();
    assert!(error.contains("arg type Str not assignable to Path"));
}

#[test]
fn explicit_path_construction_reaches_verified_ssa() {
    let source = filesystem_main(concat!(
        "unwrap-ok/\npath-from-str/\nstr/\n/tmp/verified-path\n/str\n",
        "/path-from-str\n/unwrap-ok"
    ));
    let program = compile_source(&source, "path.lkjscript", &Limits::default())
        .expect("explicit Path program");
    let operations: Vec<_> = program
        .ssa()
        .program()
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.kind {
            lkjscript_ir::InstructionKind::Runtime { operation, .. } => Some(*operation),
            _ => None,
        })
        .collect();
    assert!(operations.contains(&lkjscript_ir::RuntimeOp::PathFromStr));
    assert!(operations.contains(&lkjscript_ir::RuntimeOp::SysOpenRead));
}

#[test]
fn path_is_a_distinct_non_affine_source_type() {
    assert_eq!(
        crate::types::parse_one(&["Path".into()], 0),
        Ok((Type::Path, 1))
    );
    assert_ne!(Type::Path, Type::Str);
}

#[test]
fn accepts_only_canonical_source_extension() {
    assert!(ensure_source_path(Path::new("main.lkjscript")).is_ok());
    assert!(ensure_source_path(Path::new("main.lkjml")).is_err());
    assert!(ensure_source_path(Path::new("main")).is_err());
}

#[test]
fn public_in_memory_apis_require_canonical_relative_lkjscript_paths() {
    let source = unit_main("unit");
    for rejected in [
        "../escape.lkjscript",
        "./aliased.lkjscript",
        "src//aliased.lkjscript",
        "/absolute.lkjscript",
        "legacy.lkjml",
    ] {
        assert!(
            validate_source(&source, rejected, &Limits::default()).is_err(),
            "validate_source accepted {rejected}"
        );
        assert!(
            compile_source(&source, rejected, &Limits::default()).is_err(),
            "compile_source accepted {rejected}"
        );
    }
    validate_source(&source, "src/canonical.lkjscript", &Limits::default())
        .expect("validate canonical logical path");
    compile_source(&source, "src/canonical.lkjscript", &Limits::default())
        .expect("compile canonical logical path");
}
