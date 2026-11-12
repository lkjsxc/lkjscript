use lkjscript::diagnostics;
use lkjscript::source::{self, SourceBundle, SourceInput};
use lkjscript::{compiler, syntax};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bundle(files: &[(&str, &str)]) -> SourceBundle {
    SourceBundle {
        root: "case/main.rs".to_string(),
        library: None,
        files: files
            .iter()
            .map(|(path, text)| SourceInput {
                path: path.to_string(),
                text: text.to_string(),
            })
            .collect(),
    }
}
fn one(text: &str) -> SourceBundle {
    bundle(&[("case/main.rs", text)])
}

#[test]
fn recovery_reaches_independent_statements_items_and_nested_methods() {
    let input = one(r#"
        fn broken() { let = 1; mystery!(); let Some(x) = unknown else {}; }
        fn good() -> i64 { 42 }
        mod nested { fn broken() { other!(); } fn good() -> i64 { 7 } }
        struct Item;
        impl Item {
            const BAD: i64 = 0;
            fn broken() { third!(); }
            fn good() -> i64 { 9 }
        }
    "#);
    let report = source::parse_bundle_report(&input);
    assert_eq!(report.diagnostics.len(), 6);
    let functions: Vec<_> = report
        .program
        .functions
        .iter()
        .map(|f| (f.module.as_str(), f.name.as_str()))
        .collect();
    assert!(functions.contains(&("", "good")));
    assert!(functions.contains(&("nested", "good")));
    assert_eq!(
        functions.iter().filter(|(_, name)| *name == "good").count(),
        3
    );
    assert!(report.diagnostics.iter().all(|e| e.path == "case/main.rs"));
    let census = diagnostics::census(&report);
    assert_eq!(census.unsupported["let else"], 1);
    assert_eq!(census.unsupported["mystery!"], 1);
    assert_eq!(census.unsupported["impl item"], 1);
    assert_eq!(census.blocked["statement"], 5);
    assert_eq!(census.blocked["item"], 1);
    assert_eq!(census.blocked["semantic phase"], 1);
    assert!(compiler::compile(&input).is_err());
}

#[test]
fn loading_continues_independent_modules_after_missing_and_invalid_sources() {
    let input = bundle(&[
        ("case/main.rs", "mod missing; mod bad; mod good;"),
        ("case/bad.rs", "fn broken() { mystery!(); }"),
        ("case/good.rs", "pub fn answer() -> i64 { 42 }"),
    ]);
    let report = source::parse_bundle_report(&input);
    assert_eq!(report.diagnostics.len(), 2);
    assert_eq!(report.diagnostics[0].code, "E_MODULE_MISSING");
    assert!(
        report
            .diagnostics
            .iter()
            .any(|e| e.path == "case/bad.rs" && e.code == "E_UNSUPPORTED_MACRO")
    );
    assert!(
        report
            .program
            .functions
            .iter()
            .any(|f| f.module == "good" && f.name == "answer")
    );
    assert_eq!(report.program.files.len(), 3);
    assert!(source::parse_bundle(&input).is_err());
}

#[test]
fn poisoning_reports_blocked_semantics_instead_of_invented_name_errors() {
    let input = one("fn broken() { let x = ; x.absent(); } fn other() -> i64 { unknown }");
    let report = source::parse_bundle_report(&input);
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].code, "E_PARSE");
    assert!(report.blocked.iter().any(|r| r.kind == "semantic phase"));
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|e| e.code == "E_NAME" || e.code == "E_TYPE")
    );
    assert_eq!(compiler::compile(&input).unwrap_err().code, "E_PARSE");
}

#[test]
fn census_uses_parsed_constructs_and_ignores_comments_and_literal_text() {
    let input = one(r#"
        fn probe() {
            let text = "? if let static format! closure"; // while let dyn ?
            let byte = b'?';
            let character = '?';
            if let Some(value) = unknown {}
            while let Some(value) = unknown {}
            let result = unknown()?;
            let closure = |x: i64| x + 1;
            let string = format!("{}", 1);
            match 0 { 0 | 1 if true => 0, _ => 1 };
            for value in values {}
            let slice = &bytes[1..3];
            let repeated = vec![0; 3];
            let pointer: fn(i64) -> i64;
            loop { break 7; }
        }
    "#);
    let report = source::parse_bundle_report(&input);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let census = diagnostics::census(&report);
    for kind in [
        "char literal",
        "if let",
        "while let",
        "?",
        "closure",
        "format!",
        "or-pattern",
        "match guard",
        "collection iteration",
        "range slicing",
        "repeated Vec literal",
        "function pointer",
        "break value",
    ] {
        assert_eq!(
            census.unsupported.get(kind),
            Some(&1),
            "{kind}: {:?}",
            census.unsupported
        );
    }
    assert_eq!(census.unsupported.len(), 13);
    assert!(census.blocked.is_empty());
}

#[test]
fn unknown_lexical_boundaries_and_unbalanced_input_are_blocked() {
    let lexical = source::parse_bundle_report(&one("fn before() {} /* missing"));
    assert_eq!(lexical.diagnostics.len(), 1);
    assert_eq!(lexical.diagnostics[0].code, "E_LEX");
    assert_eq!(diagnostics::census(&lexical).blocked["file"], 1);
    let unbalanced = source::parse_bundle_report(&one("fn broken() { let x = missing("));
    assert!(!unbalanced.diagnostics.is_empty());
    assert!(unbalanced.blocked.iter().any(|r| r.kind == "item"));
    assert!(source::parse_bundle(&one("fn broken() { let x = missing(")).is_err());
}

#[test]
fn the_same_recovering_parser_preserves_valid_compilation() {
    let text = "fn square(x: i64) -> i64 { x * x } fn probe() -> i64 { square(6) + 6 }";
    let report = source::parse_bundle_report(&one(text));
    assert!(report.diagnostics.is_empty());
    assert!(report.blocked.is_empty());
    assert!(diagnostics::census(&report).unsupported.is_empty());
    assert_eq!(syntax::parse(text).unwrap().functions.len(), 2);
    assert!(compiler::compile(&one(text)).is_ok());
}

#[test]
fn build_does_not_emit_an_artifact_from_recovered_syntax() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/diagnostic-tests");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{}-invalid.rs", std::process::id()));
    let output_path = path.with_extension("wasm");
    let _ = fs::remove_file(&output_path);
    fs::write(&path, "fn broken() { let = 1; } fn probe() -> i64 { 42 }").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .arg("build")
        .arg(&path)
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E_PARSE"));
    assert!(!output_path.exists());
}

#[test]
fn missing_semicolon_does_not_hide_later_independent_statements() {
    let report = source::parse_bundle_report(&one(
        "fn broken() { let x = 1 let y = ; let z = ; } fn good() -> i64 { 42 }",
    ));
    assert_eq!(report.diagnostics.len(), 3);
    assert!(report.diagnostics[0].message.contains("expected `;`"));
    assert_eq!(diagnostics::census(&report).blocked["statement"], 3);
    assert!(report.program.functions.iter().any(|f| f.name == "good"));
}

#[test]
fn semantic_definition_errors_accumulate_without_poisoning_call_signatures() {
    let input = bundle(&[
        (
            "case/main.rs",
            "mod child; fn first() -> i64 { true } fn dependent() -> i64 { first() }",
        ),
        ("case/child.rs", "pub fn second() -> bool { 1i64 }"),
    ]);
    let parsed = source::parse_bundle(&input).unwrap();
    let errors = match lkjscript::elaborate::elaborate_all(&parsed) {
        Ok(_) => panic!("invalid definitions accepted"),
        Err(errors) => errors,
    };
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors.iter().all(|error| error.code == "E_TYPE"
            && error.message.contains("function result type differs"))
    );
    assert_ne!(errors[0].file, errors[1].file);
    assert_eq!(compiler::compile(&input).unwrap_err().path, "case/main.rs");
}

#[test]
fn invalid_shared_signatures_block_semantic_recovery_before_body_checks() {
    let parsed = source::parse_bundle(&one(
        "fn first(x: Missing) -> i64 { true } fn second() -> bool { 1i64 }",
    ))
    .unwrap();
    let errors = match lkjscript::elaborate::elaborate_all(&parsed) {
        Ok(_) => panic!("unresolved signature accepted"),
        Err(errors) => errors,
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "E_NAME");
    assert!(errors[0].message.contains("Missing"));
}

fn uninitialized_function(name: &str, file: usize, line: usize) -> lkjscript::k::Function {
    use lkjscript::k;
    k::Function {
        file,
        name: name.to_string(),
        params: 0,
        result: k::I64,
        return_source: None,
        locals: vec![k::Local {
            name: "value".to_string(),
            ty: k::I64,
            mutable: false,
        }],
        body: vec![k::Instruction {
            op: k::Op::Return { src: 0 },
            line,
        }],
        test: false,
    }
}

#[test]
fn independent_k_body_errors_never_construct_checked_program_authority() {
    use lkjscript::{check, k};
    let program = k::Program {
        types: k::primitives(),
        functions: vec![
            uninitialized_function("first", 0, 7),
            uninitialized_function("second", 1, 9),
        ],
        destructors: Vec::new(),
    };
    let errors = match check::check_all(program.clone()) {
        Ok(_) => panic!("invalid K bodies accepted"),
        Err(errors) => errors,
    };
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|error| error.code == "E_MOVE"));
    assert_eq!((errors[0].file, errors[0].line), (0, 7));
    assert_eq!((errors[1].file, errors[1].line), (1, 9));
    let first = match check::check(program) {
        Ok(_) => panic!("single-error API bypassed failure"),
        Err(error) => error,
    };
    assert_eq!(first, errors[0]);
}

#[test]
fn malformed_k_structure_blocks_all_body_checks() {
    use lkjscript::{check, k};
    let mut first = uninitialized_function("bad_signature", 0, 7);
    first.locals[0].ty = usize::MAX;
    let program = k::Program {
        types: k::primitives(),
        functions: vec![first, uninitialized_function("blocked", 1, 9)],
        destructors: Vec::new(),
    };
    let errors = match check::check_all(program) {
        Ok(_) => panic!("invalid common structure accepted"),
        Err(errors) => errors,
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "E_K");
    assert!(errors[0].message.contains("unknown local type"));
}

#[test]
fn cli_collects_independent_errors_marks_blocking_and_never_emits() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("diagnostic-cli");
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("independent.rs");
    let output = dir.join("invalid.wasm");
    std::fs::write(
        &input,
        "fn first() -> i64 { true }\nfn second() -> bool { 1i64 }\n",
    )
    .unwrap();
    if output.exists() {
        std::fs::remove_file(&output).unwrap();
    }
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let errors = String::from_utf8(result.stderr).unwrap();
    assert_eq!(errors.matches(": E_TYPE ").count(), 2, "{errors}");
    assert!(errors.contains("E_BLOCKED"), "{errors}");
    assert!(!output.exists());
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .arg("check")
        .arg("--census")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("remaining semantic analysis\t1\tblocked")
    );
}

#[test]
fn census_counts_semantic_diagnostics_and_their_parsed_site_once() {
    for (text, kind) in [
        (
            "fn consume(callback: fn(i64) -> i64) {}",
            "function pointer",
        ),
        ("fn probe() { let closure = |x: i64| x; }", "closure"),
        (
            "fn probe() { let v: Vec<i64> = Vec::new(); for x in &v {} }",
            "collection iteration",
        ),
    ] {
        let mut report = source::parse_bundle_report(&one(text));
        assert!(report.diagnostics.is_empty());
        let errors = match lkjscript::elaborate::elaborate_all(&report.program) {
            Ok(_) => panic!("unsupported definition accepted: {kind}"),
            Err(errors) => errors,
        };
        assert!(
            errors.iter().any(|error| error.code == "E_UNSUPPORTED"),
            "{kind}: {errors:?}"
        );
        report.diagnostics.extend(errors);
        let census = diagnostics::census(&report);
        assert_eq!(
            census.unsupported.get(kind),
            Some(&1),
            "{kind}: {:?}",
            census.unsupported
        );
        assert_eq!(
            census.unsupported.values().sum::<usize>(),
            1,
            "{kind}: {:?}",
            census.unsupported
        );
    }
}
