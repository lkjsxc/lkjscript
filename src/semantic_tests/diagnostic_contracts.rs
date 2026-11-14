use crate::diagnostics;
use crate::source::{self, SourceBundle, SourceInput};
use crate::{frontend as compiler, syntax};

fn bundle(files: &[(String, String)]) -> SourceBundle {
    let mut inputs = Vec::new();
    for (path, text) in files {
        inputs.push(SourceInput {
            path: path.clone(),
            text: text.clone(),
        });
    }
    SourceBundle {
        root: "case/main.rs".to_string(),
        library: None,
        files: inputs,
    }
}
fn one(text: &str) -> SourceBundle {
    bundle(&vec![("case/main.rs".to_string(), text.to_string())])
}

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
    assert_eq!(report.diagnostics.len(), 5);
    let mut good = 0usize;
    let mut root_good = false;
    let mut nested_good = false;
    for function in &report.program.functions {
        if function.name == "good" {
            good += 1;
            if function.module.is_empty() {
                root_good = true;
            }
            if function.module == "nested" {
                nested_good = true;
            }
        }
    }
    assert!(root_good && nested_good && good == 3usize);
    assert!(report.diagnostics.iter().all(|e| e.path == "case/main.rs"));
    assert_eq!(blocked_count(&report.blocked, "statement"), 4);
    assert!(report.blocked.iter().any(|region| region.kind == "item"));
    assert!(
        report
            .blocked
            .iter()
            .any(|region| region.kind == "semantic phase")
    );
    assert!(compiler::check_bundle(&input).is_err());
}

fn loading_continues_independent_modules_after_missing_and_invalid_sources() {
    let input = bundle(&vec![
        (
            "case/main.rs".to_string(),
            "mod missing; mod bad; mod good;".to_string(),
        ),
        (
            "case/bad.rs".to_string(),
            "fn broken() { mystery!(); }".to_string(),
        ),
        (
            "case/good.rs".to_string(),
            "pub fn answer() -> i64 { 42 }".to_string(),
        ),
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
    assert_eq!(compiler::check_bundle(&input).unwrap_err().code, "E_PARSE");
}

fn unsupported_diagnostics_use_parsed_constructs_and_ignore_literal_text() {
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
    let mut report = source::parse_bundle_report(&input);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    diagnostics::complete_unsupported(&mut report);
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].code, "E_UNSUPPORTED");
    assert_eq!(
        report.diagnostics[0].message,
        "unsupported construct: function pointer"
    );
    assert!(report.blocked.is_empty());
}

fn unknown_lexical_boundaries_and_unbalanced_input_are_blocked() {
    let lexical = source::parse_bundle_report(&one("fn before() {} /* missing"));
    assert_eq!(lexical.diagnostics.len(), 1);
    assert_eq!(lexical.diagnostics[0].code, "E_LEX");
    assert_eq!(blocked_count(&lexical.blocked, "file"), 1);
    let unbalanced = source::parse_bundle_report(&one("fn broken() { let x = missing("));
    assert!(!unbalanced.diagnostics.is_empty());
    assert!(unbalanced.blocked.iter().any(|r| r.kind == "item"));
    assert!(source::parse_bundle(&one("fn broken() { let x = missing(")).is_err());
}

fn the_same_recovering_parser_preserves_valid_compilation() {
    let text = "fn square(x: i64) -> i64 { x * x } fn probe() -> i64 { square(6) + 6 }";
    let report = source::parse_bundle_report(&one(text));
    assert!(report.diagnostics.is_empty());
    assert!(report.blocked.is_empty());
    assert_eq!(syntax::parse(text).unwrap().functions.len(), 2);
    assert!(compiler::check_bundle(&one(text)).is_ok());
}

fn missing_semicolon_does_not_hide_later_independent_statements() {
    let report = source::parse_bundle_report(&one(
        "fn broken() { let x = 1 let y = ; let z = ; } fn good() -> i64 { 42 }",
    ));
    assert_eq!(report.diagnostics.len(), 3);
    assert!(report.diagnostics[0].message.contains("expected `;`"));
    assert_eq!(blocked_count(&report.blocked, "statement"), 3);
    assert!(report.program.functions.iter().any(|f| f.name == "good"));
}

fn semantic_definition_errors_accumulate_without_poisoning_call_signatures() {
    let input = bundle(&vec![
        (
            "case/main.rs".to_string(),
            "mod child; fn first() -> i64 { true } fn dependent() -> i64 { first() }".to_string(),
        ),
        (
            "case/child.rs".to_string(),
            "pub fn second() -> bool { 1i64 }".to_string(),
        ),
    ]);
    let parsed = source::parse_bundle(&input).unwrap();
    let errors = match crate::elaborate::elaborate_all(&parsed) {
        Ok(_) => panic!("invalid definitions accepted"),
        Err(errors) => errors,
    };
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors.iter().all(|error| error.code == "E_TYPE"
            && error.message.contains("function result type differs"))
    );
    assert_ne!(errors[0].file, errors[1].file);
    assert_eq!(
        compiler::check_bundle(&input).unwrap_err().path,
        "case/main.rs"
    );
}

fn invalid_shared_signatures_block_semantic_recovery_before_body_checks() {
    let parsed = source::parse_bundle(&one(
        "fn first(x: Missing) -> i64 { true } fn second() -> bool { 1i64 }",
    ))
    .unwrap();
    let errors = match crate::elaborate::elaborate_all(&parsed) {
        Ok(_) => panic!("unresolved signature accepted"),
        Err(errors) => errors,
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "E_NAME");
    assert!(errors[0].message.contains("Missing"));
}

fn uninitialized_function(name: &str, file: usize, line: usize) -> crate::ir::Function {
    use crate::ir;
    ir::Function {
        file,
        name: name.to_string(),
        params: 0,
        result: ir::I64,
        return_source: None,
        locals: vec![ir::Local {
            name: "value".to_string(),
            ty: ir::I64,
            mutable: false,
        }],
        body: vec![ir::Instruction {
            column: 1,
            op: ir::Op::Return { src: 0 },
            line,
        }],
        test: false,
    }
}

fn independent_ir_body_errors_never_construct_checked_program_authority() {
    use crate::{check, ir};
    let program = ir::Program {
        types: ir::primitives(),
        functions: vec![
            uninitialized_function("first", 0, 7),
            uninitialized_function("second", 1, 9),
        ],
        destructors: Vec::new(),
    };
    let errors = match check::check_all(program.clone()) {
        Ok(_) => panic!("invalid IR bodies accepted"),
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

fn malformed_ir_structure_blocks_all_body_checks() {
    use crate::{check, ir};
    let mut first = uninitialized_function("bad_signature", 0, 7);
    first.locals[0].ty = !0usize;
    let program = ir::Program {
        types: ir::primitives(),
        functions: vec![first, uninitialized_function("blocked", 1, 9)],
        destructors: Vec::new(),
    };
    let errors = match check::check_all(program) {
        Ok(_) => panic!("invalid common structure accepted"),
        Err(errors) => errors,
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "E_IR");
    assert!(errors[0].message.contains("unknown local type"));
}

fn unsupported_diagnostics_preserve_specific_existing_errors() {
    for (text, kind) in vec![
        (
            "fn consume(callback: fn(i64) -> i64) {}".to_string(),
            "function pointer".to_string(),
        ),
        (
            "fn probe() { let v: Vec<i64> = Vec::new(); for x in &v {} }".to_string(),
            "source library module $std::iter is not loaded".to_string(),
        ),
    ] {
        let mut report = source::parse_bundle_report(&one(&text));
        assert!(report.diagnostics.is_empty());
        let errors = match crate::elaborate::elaborate_all(&report.program) {
            Ok(_) => panic!("unsupported definition accepted: {kind}"),
            Err(errors) => errors,
        };
        assert!(
            errors.iter().any(|error| error.code == "E_UNSUPPORTED"),
            "{kind}: {errors:?}"
        );
        for error in errors {
            report.diagnostics.push(error);
        }
        let mut existing: Vec<String> = Vec::new();
        for problem in &report.diagnostics {
            existing.push(problem.message.clone());
        }
        diagnostics::complete_unsupported(&mut report);
        let mut current: Vec<String> = Vec::new();
        for problem in &report.diagnostics {
            current.push(problem.message.clone());
        }
        assert!(current == existing);
    }
}

fn unsized_source_locals_keep_their_module_and_construction_location() {
    for (text, line, column, local) in vec![
        (
            "\nfn consume(text: &str) {\n    let value = *text;\n}\n".to_string(),
            3,
            17,
            "$".to_string(),
        ),
        (
            "\n\nfn consume(value: str) {}\n".to_string(),
            3usize,
            1usize,
            "value".to_string(),
        ),
    ] {
        let input = bundle(&vec![
            ("case/main.rs".to_string(), "mod nested;".to_string()),
            ("case/nested.rs".to_string(), text.clone()),
        ]);
        let problem = compiler::check_bundle(&input).err().unwrap();
        assert_eq!(problem.code, "E_UNSUPPORTED");
        assert_eq!(problem.path, "case/nested.rs");
        assert_eq!((problem.line, problem.column), (line, column));
        assert!(problem.message.contains(&format!("local {local} (str)")));
        assert!(problem.message.contains("function nested::consume"));
    }
}

fn blocked_count(regions: &[diagnostics::BlockedRegion], kind: &str) -> usize {
    let mut count = 0usize;
    for region in regions {
        if region.kind == kind {
            count += 1;
        }
    }
    count
}

pub fn run(world: &mut World, trace: bool) -> usize {
    if trace {
        world
            .stdout()
            .write(b"recovery_reaches_independent_statements_items_and_nested_methods\n");
    }
    recovery_reaches_independent_statements_items_and_nested_methods();
    if trace {
        world
            .stdout()
            .write(b"loading_continues_independent_modules_after_missing_and_invalid_sources\n");
    }
    loading_continues_independent_modules_after_missing_and_invalid_sources();
    if trace {
        world
            .stdout()
            .write(b"poisoning_reports_blocked_semantics_instead_of_invented_name_errors\n");
    }
    poisoning_reports_blocked_semantics_instead_of_invented_name_errors();
    if trace {
        world
            .stdout()
            .write(b"unsupported_diagnostics_use_parsed_constructs_and_ignore_literal_text\n");
    }
    unsupported_diagnostics_use_parsed_constructs_and_ignore_literal_text();
    if trace {
        world
            .stdout()
            .write(b"unknown_lexical_boundaries_and_unbalanced_input_are_blocked\n");
    }
    unknown_lexical_boundaries_and_unbalanced_input_are_blocked();
    if trace {
        world
            .stdout()
            .write(b"the_same_recovering_parser_preserves_valid_compilation\n");
    }
    the_same_recovering_parser_preserves_valid_compilation();
    if trace {
        world
            .stdout()
            .write(b"missing_semicolon_does_not_hide_later_independent_statements\n");
    }
    missing_semicolon_does_not_hide_later_independent_statements();
    if trace {
        world
            .stdout()
            .write(b"semantic_definition_errors_accumulate_without_poisoning_call_signatures\n");
    }
    semantic_definition_errors_accumulate_without_poisoning_call_signatures();
    if trace {
        world
            .stdout()
            .write(b"invalid_shared_signatures_block_semantic_recovery_before_body_checks\n");
    }
    invalid_shared_signatures_block_semantic_recovery_before_body_checks();
    if trace {
        world
            .stdout()
            .write(b"independent_ir_body_errors_never_construct_checked_program_authority\n");
    }
    independent_ir_body_errors_never_construct_checked_program_authority();
    if trace {
        world
            .stdout()
            .write(b"malformed_ir_structure_blocks_all_body_checks\n");
    }
    malformed_ir_structure_blocks_all_body_checks();
    if trace {
        world
            .stdout()
            .write(b"unsupported_diagnostics_preserve_specific_existing_errors\n");
    }
    unsupported_diagnostics_preserve_specific_existing_errors();
    if trace {
        world
            .stdout()
            .write(b"unsized_source_locals_keep_their_module_and_construction_location\n");
    }
    unsized_source_locals_keep_their_module_and_construction_location();
    13usize
}
