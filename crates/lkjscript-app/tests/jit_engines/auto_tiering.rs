use crate::canonical::{compile, execution, f64_loop};
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_jit::{JitConfig, JitSession, TierState};
use lkjscript_vm::{run_chunk, run_chunk_auto};

#[test]
fn auto_group_reference_helper_remains_vm_entry_ineligible() {
    let source = "def/\nname/\ntext\n/name\nfn/\nsig/\n->\nStr\n/sig\nparams/\n/params\nempty-str/\n/empty-str\n/fn\n/def\ndef/\nname/\nsize\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\nstr-len/\ntext/\n/text\n/str-len\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\ndo/\nsize/\n/size\ntext/\n/text\nsize/\n/size\n/do\n/main\n";
    let program = compile(source, "auto-reference-helper.lkjscript");
    let mut config = JitConfig::default();
    config.auto_threshold = 1;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
        session,
    );
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(0)));
    let helper = stats
        .functions
        .iter()
        .find(|function| function.name() == "text")
        .expect("reference helper tier record");
    assert!(!helper.auto_entry_eligible());
    assert_ne!(helper.state(), TierState::BaselineNative);
    assert!(
        helper.native_entries() > 0,
        "native direct calls remain supported"
    );
    assert_eq!(stats.compile_failures, 0);
}

#[test]
fn auto_path_helper_remains_vm_only() {
    let source = concat!(
        "def/\nname/\npath\n/name\nfn/\nsig/\n->\nPath\n/sig\nparams/\n/params\n",
        "unwrap-ok/\npath-from-str/\nstr/\n/tmp/auto-path\n/str\n/path-from-str\n",
        "/unwrap-ok\n/fn\n/def\nmain/\nsig/\n->\nPath\n/sig\npath/\n/path\n/main\n",
    );
    let program = compile(source, "auto-path.lkjscript");
    let mut config = JitConfig::default();
    config.auto_threshold = 1;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
        session,
    );
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_path_bytes() == Some(b"/tmp/auto-path")
    ));
    let path = stats
        .functions
        .iter()
        .find(|function| function.name() == "path")
        .expect("Path helper tier record");
    assert_eq!(path.state(), TierState::VmOnly);
    assert!(!path.auto_entry_eligible());
    assert_eq!(path.attempts(), 0);
    assert_eq!(stats.compile_failures, 0);
}

#[test]
fn auto_compiles_for_later_calls_and_suppresses_unsupported_retry() {
    let program = compile(&f64_loop(), "auto.lkjscript");
    let mut config = JitConfig::default();
    config.auto_threshold = 2;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
        session,
    );
    assert_eq!(
        execution(outcome),
        execution(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default()
        ))
    );
    let step = stats
        .functions
        .iter()
        .find(|function| function.name() == "step")
        .expect("step tier record");
    assert_eq!(step.state(), TierState::BaselineNative);
    assert_eq!(step.attempts(), 1);
    assert!(step.native_entries() > 0);
    assert!(stats.vm_fallbacks >= 2);

    let allocation = "def/\nname/\nallocate\n/name\nfn/\nsig/\n->\nStr\n/sig\nparams/\n/params\nempty-str/\n/empty-str\n/fn\n/def\nmain/\nsig/\n->\nStr\n/sig\ndo/\nallocate/\n/allocate\nallocate/\n/allocate\nallocate/\n/allocate\n/do\n/main\n";
    let program = compile(allocation, "auto-unsupported.lkjscript");
    let mut config = JitConfig::default();
    config.auto_threshold = 1;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
        session,
    );
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_str() == Some("")));
    let allocate = stats
        .functions
        .iter()
        .find(|function| function.name() == "allocate")
        .expect("allocation tier record");
    assert_eq!(allocate.state(), TierState::VmOnly);
    assert!(!allocate.auto_entry_eligible());
    assert_eq!(allocate.attempts(), 0);
    assert_eq!(allocate.native_entries(), 0);
    assert_eq!(stats.compile_failures, 0);
}
