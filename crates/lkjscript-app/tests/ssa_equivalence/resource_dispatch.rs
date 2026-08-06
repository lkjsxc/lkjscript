use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalResourcePolicy};
use lkjscript_vm::run_chunk;

#[test]
fn fake_evaluator_and_reference_vm_run_typed_resource_acquisition_and_cleanup() {
    let path = format!(
        "/tmp/lkjscript-implicit-resource-drop-{}",
        std::process::id()
    );
    std::fs::write(&path, b"resource").expect("create implicit-drop fixture");
    let source = format!(
        concat!(
            "main/\nsig/\ninputs/\ncapability/\nfile-system\n/capability\n/inputs\n",
            "output/\nunit\n/output\n/sig\nparams/\nfile-system\ncapability/\n",
            "file-system\n/capability\n/params\nlet/\nbind/\nreader\nunwrap-ok/\n",
            "open-file-reader/\nfile-system\nunwrap-ok/\nconvert-string-to-path/\n",
            "string-literal/\n{}\n/string-literal\n/convert-string-to-path\n/unwrap-ok\n",
            "/open-file-reader\n/unwrap-ok\n/bind\nunit\n/let\n/main\n"
        ),
        path
    );
    let program = compile_source(&source, "implicit-resource-drop.lkjscript")
        .expect("compile implicit resource close");
    let evaluator_config = EvalConfig {
        capabilities: vec![lkjscript_core::CapabilityKind::FileSystem],
        ..EvalConfig::default()
    };
    assert!(matches!(
        evaluate(program.ssa(), &evaluator_config),
        EvalOutcome::Returned(lkjscript_ir::EvalValue::Unit)
    ));
    let failed_evaluator_config = EvalConfig {
        resource_policy: EvalResourcePolicy {
            fail_acquisition: Some(lkjscript_core::ResourceKind::FileReader),
            ..EvalResourcePolicy::default()
        },
        ..evaluator_config.clone()
    };
    assert!(matches!(
        evaluate(program.ssa(), &failed_evaluator_config),
        EvalOutcome::Trapped(_)
    ));

    let explicit_close_source = format!(
        concat!(
            "main/\nsig/\ninputs/\ncapability/\nfile-system\n/capability\n/inputs\n",
            "output/\nunit\n/output\n/sig\nparams/\nfile-system\ncapability/\n",
            "file-system\n/capability\n/params\nlet/\nbind/\nreader\nunwrap-ok/\n",
            "open-file-reader/\nfile-system\nunwrap-ok/\nconvert-string-to-path/\n",
            "string-literal/\n{}\n/string-literal\n/convert-string-to-path\n/unwrap-ok\n",
            "/open-file-reader\n/unwrap-ok\n/bind\nunwrap-ok/\ndrop/\nreader\n",
            "/drop\n/unwrap-ok\n/let\n/main\n"
        ),
        path
    );
    let explicit_close = compile_source(
        &explicit_close_source,
        "explicit-evaluator-resource-close.lkjscript",
    )
    .expect("compile explicit evaluator resource close");
    assert!(matches!(
        evaluate(explicit_close.ssa(), &evaluator_config),
        EvalOutcome::Returned(lkjscript_ir::EvalValue::Unit)
    ));
    let failed_close_config = EvalConfig {
        resource_policy: EvalResourcePolicy {
            fail_close: Some(lkjscript_core::ResourceKind::FileReader),
            ..EvalResourcePolicy::default()
        },
        ..evaluator_config.clone()
    };
    assert!(matches!(
        evaluate(explicit_close.ssa(), &failed_close_config),
        EvalOutcome::Trapped(_)
    ));

    let stdin_source = concat!(
        "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\n",
        "output/\nbool\n/output\n/sig\nparams/\nstdio\ncapability/\nstdio\n",
        "/capability\n/params\nunwrap-ok/\nis-terminal/\nstandard-input/\nstdio\n",
        "/standard-input\n/is-terminal\n/unwrap-ok\n/main\n"
    );
    let stdin = compile_source(stdin_source, "evaluator-standard-input.lkjscript")
        .expect("compile evaluator standard input");
    let stdin_config = EvalConfig {
        capabilities: vec![lkjscript_core::CapabilityKind::Stdio],
        ..EvalConfig::default()
    };
    assert!(matches!(
        evaluate(stdin.ssa(), &stdin_config),
        EvalOutcome::Returned(lkjscript_ir::EvalValue::Bool(false))
    ));

    let sqlite_source = concat!(
        "main/\nsig/\ninputs/\ncapability/\nsqlite\n/capability\n/inputs\n",
        "output/\nunit\n/output\n/sig\nparams/\nsqlite\ncapability/\nsqlite\n",
        "/capability\n/params\nlet/\nbind/\ndatabase\nunwrap-ok/\nopen-sqlite/\n",
        "sqlite\nunwrap-ok/\nconvert-string-to-path/\nstring-literal/\n/tmp/fake.sqlite\n",
        "/string-literal\n/convert-string-to-path\n/unwrap-ok\n0\n/open-sqlite\n",
        "/unwrap-ok\n/bind\nlet/\nbind/\nstatement\nunwrap-ok/\nprepare-sqlite/\n",
        "database\nstring-literal/\nSELECT 1\n/string-literal\n/prepare-sqlite\n",
        "/unwrap-ok\n/bind\ndo/\nunwrap-ok/\nfinalize-sqlite-statement/\nstatement\n",
        "/finalize-sqlite-statement\n/unwrap-ok\nunwrap-ok/\nclose-sqlite/\ndatabase\n",
        "/close-sqlite\n/unwrap-ok\n/do\n/let\n/let\n/main\n"
    );
    let sqlite = compile_source(sqlite_source, "evaluator-fake-sqlite.lkjscript")
        .expect("compile evaluator fake SQLite lifecycle");
    let sqlite_config = EvalConfig {
        capabilities: vec![lkjscript_core::CapabilityKind::Sqlite],
        ..EvalConfig::default()
    };
    assert!(matches!(
        evaluate(sqlite.ssa(), &sqlite_config),
        EvalOutcome::Returned(lkjscript_ir::EvalValue::Unit)
    ));
    let failed_statement_config = EvalConfig {
        resource_policy: EvalResourcePolicy {
            fail_acquisition: Some(lkjscript_core::ResourceKind::SqliteStatement),
            ..EvalResourcePolicy::default()
        },
        ..sqlite_config
    };
    assert!(matches!(
        evaluate(sqlite.ssa(), &failed_statement_config),
        EvalOutcome::Trapped(_)
    ));

    let outcome = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs {
            arguments: Vec::new(),
            capabilities: vec![lkjscript_core::CapabilityKind::FileSystem],
            host: lkjscript_host::HostEnvironment::default(),
        },
        &ExecutionConfig::default(),
    );
    let _removed = std::fs::remove_file(path);
    assert!(
        matches!(outcome, ExecutionOutcome::Returned(_)),
        "{outcome:?}"
    );
}
