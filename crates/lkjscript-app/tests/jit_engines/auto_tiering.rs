use crate::canonical::{compile, execution, f64_loop};
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_jit::{JitConfig, JitSession, TierState};
use lkjscript_vm::{run_chunk, run_chunk_auto};

fn generated_scalar_helpers(count: usize) -> String {
    let mut source = String::new();
    for index in 0..count {
        source.push_str("def/\nname/\n");
        source.push_str(&format!("helper-{index}\n"));
        source.push_str("/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nvalue\ni64\n/params\nvalue\n");
        source.push_str("/fn\n/def\n");
    }
    let hot = count - 1;
    source.push_str("main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ndo/\n");
    for _ in 0..3 {
        source.push_str(&format!("helper-{hot}/\n99\n/helper-{hot}\n"));
    }
    source.push_str("/do\n/main\n");
    source
}

#[test]
fn auto_executes_hot_high_function_id_natively() {
    let program = compile(
        &generated_scalar_helpers(100),
        "auto-high-function.lkjscript",
    );
    let mut config = JitConfig::default();
    config.auto_threshold = 2;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
        session,
    );
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(99)));
    let hot = stats
        .functions
        .iter()
        .find(|function| function.name() == "helper-99")
        .expect("hot high-ID helper tier record");
    assert!(
        hot.function().raw() > 63,
        "generated helper must cross the former source-ID ceiling"
    );
    assert!(hot.auto_entry_eligible());
    assert_eq!(hot.state(), TierState::BaselineNative, "{hot:?}");
    assert!(hot.native_entries() > 0);
    assert_eq!(stats.compile_failures, 0);
}

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
        &ExecutionPolicy::unrestricted(),
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
        &ExecutionPolicy::unrestricted(),
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
        &ExecutionPolicy::unrestricted(),
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
        &ExecutionPolicy::unrestricted(),
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
        &ExecutionPolicy::unrestricted(),
        session,
    );
    assert_eq!(
        execution(outcome),
        execution(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted()
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
        &ExecutionPolicy::unrestricted(),
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
