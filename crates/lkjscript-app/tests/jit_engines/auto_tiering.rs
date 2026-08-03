use crate::canonical::{compile, execution, f64_loop};
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_jit::{JitConfig, JitSession, TierState};
use lkjscript_vm::{run_chunk, run_chunk_auto};

#[test]
fn auto_group_structural_string_helper_remains_vm_entry_ineligible() {
    let source = "def/\nname/\ntext\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nstring\n/output\n/sig\nparams/\n/params\nempty-string/\n/empty-string\n/fn\n/def\ndef/\nname/\nsize\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\n/params\nstring-byte-length/\ntext/\n/text\n/string-byte-length\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ndo/\nsize/\n/size\ntext/\n/text\nsize/\n/size\n/do\n/main\n";
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
    assert_eq!(helper.native_entries(), 0);
    assert_eq!(stats.compile_failures, 0);
}

#[test]
fn residual_witness_function_is_direct_call_only_in_auto_mode() {
    let source = include_str!("../fixtures/sealed-placement.lkjscript");
    let program = compile(source, "sealed-placement.lkjscript");
    let mut config = JitConfig::default();
    config.auto_threshold = 1;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
        session,
    );
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(42)));
    let generic = stats
        .functions
        .iter()
        .find(|function| function.name() == "select-owner")
        .expect("residual generic tier record");
    assert!(!generic.auto_entry_eligible());
    assert_eq!(generic.native_entries(), 0);
}

#[test]
fn scalar_function_beyond_entry_abi_arity_remains_vm_only() {
    let source = concat!(
        "def/\nname/\nadd-four\n/name\nfn/\nsig/\ninputs/\ni64\ni64\ni64\ni64\n/inputs\n",
        "output/\ni64\n/output\n/sig\nparams/\na\ni64\nb\ni64\nc\ni64\nd\ni64\n/params\n",
        "add/\na\nadd/\nb\nadd/\nc\nd\n/add\n/add\n/add\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nadd-four/\n1\n2\n3\n4\n/add-four\n/main\n",
    );
    let program = compile(source, "auto-arity.lkjscript");
    let mut config = JitConfig::default();
    config.auto_threshold = 1;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
        session,
    );
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(10)));
    let function = stats
        .functions
        .iter()
        .find(|function| function.name() == "add-four")
        .expect("four-argument tier record");
    assert!(!function.auto_entry_eligible());
    assert_eq!(function.native_entries(), 0);
}

#[test]
fn auto_path_helper_remains_vm_only() {
    let source = concat!(
        "def/\nname/\nmake-path\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\npath\n/output\n/sig\nparams/\n/params\n",
        "unwrap-ok/\nconvert-string-to-path/\nstring-literal/\n/tmp/auto-path\n/string-literal\n/convert-string-to-path\n",
        "/unwrap-ok\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\npath\n/output\n/sig\nmake-path/\n/make-path\n/main\n",
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
        .find(|function| function.name() == "make-path")
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

    let allocation = "def/\nname/\nallocate\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nstring\n/output\n/sig\nparams/\n/params\nempty-string/\n/empty-string\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\nstring\n/output\n/sig\ndo/\nallocate/\n/allocate\nallocate/\n/allocate\nallocate/\n/allocate\n/do\n/main\n";
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
