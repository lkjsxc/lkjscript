use crate::canonical::compile;
use lkjscript_core::{CapabilityKind, ExecutionOutcome, ExecutionPolicy};
use lkjscript_jit::{
    execute_forced_with_capabilities, execute_optimizing_with_capabilities, JitConfig, JitStats,
};
use lkjscript_native::RuntimeCallSlot;
use lkjscript_vm::{run_chunk, ExecutionInputs};

#[test]
fn borrowed_standard_input_runs_in_forced_noncollecting_resource_island() {
    let source = concat!(
        "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\n",
        "output/\nunit\n/output\n/sig\nparams/\nstdio\ncapability/\nstdio\n",
        "/capability\n/params\ndo/\nstandard-input/\nstdio\n/standard-input\n",
        "standard-input/\nstdio\n/standard-input\nunit\n/do\n/main\n",
    );
    let program = compile(source, "native-resource-island.lkjscript");
    let capabilities = [CapabilityKind::Stdio];
    let vm = run_chunk(
        program.bytecode(),
        &ExecutionInputs {
            arguments: Vec::new(),
            capabilities: capabilities.to_vec(),
            host: lkjscript_host::HostEnvironment::portable(),
        },
        &ExecutionPolicy::unrestricted(),
    );
    assert!(matches!(vm, ExecutionOutcome::Returned(value) if value.is_unit()));

    let baseline = execute_forced_with_capabilities(
        program.ssa(),
        &capabilities,
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("forced baseline borrowed standard input");
    let proof = execute_optimizing_with_capabilities(
        program.ssa(),
        &capabilities,
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("forced proof borrowed standard input");
    assert!(matches!(baseline.outcome, ExecutionOutcome::Returned(value) if value.is_unit()));
    assert!(matches!(proof.outcome, ExecutionOutcome::Returned(value) if value.is_unit()));
    assert_resource_metrics(&baseline.stats);
    assert_resource_metrics(&proof.stats);
}

#[test]
fn resource_island_rejects_unsupported_and_legacy_reachable_operations() {
    let prefix = concat!(
        "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\n",
        "output/\nunit\n/output\n/sig\nparams/\nstdio\ncapability/\nstdio\n",
        "/capability\n/params\ndo/\n",
    );
    let cases = [
        (
            "unsupported-resource-op.lkjscript",
            "is-terminal/\nstandard-input/\nstdio\n/standard-input\n/is-terminal\n",
        ),
        (
            "legacy-reachable-resource-group.lkjscript",
            "empty-string/\n/empty-string\n",
        ),
    ];
    for (name, body) in cases {
        let source = format!("{prefix}{body}unit\n/do\n/main\n");
        let program = compile(&source, name);
        for result in [
            execute_forced_with_capabilities(
                program.ssa(),
                &[CapabilityKind::Stdio],
                &ExecutionPolicy::unrestricted(),
                JitConfig::default(),
            ),
            execute_optimizing_with_capabilities(
                program.ssa(),
                &[CapabilityKind::Stdio],
                &ExecutionPolicy::unrestricted(),
                JitConfig::default(),
            ),
        ] {
            let error = result.expect_err("resource group must reject before generated entry");
            assert_eq!(error.code(), lkjscript_jit::FailureCode::UnsupportedType);
        }
    }
}

fn assert_resource_metrics(stats: &JitStats) {
    assert!(stats.native_entries > 0);
    assert_eq!(stats.vm_fallbacks, 0);
    assert_eq!(stats.vm_to_native_transitions, 0);
    assert_eq!(stats.native_to_vm_transitions, 0);
    assert_eq!(stats.resource_runtime_calls, 2);
    assert_eq!(stats.runtime_heap_attempts, 0);
    assert_eq!(stats.native_resources.reservations, 1);
    assert_eq!(stats.native_resources.borrowed_installs, 1);
    assert_eq!(stats.native_resources.borrowed_reuses, 1);
    assert_eq!(stats.native_resources.borrowed_removals, 1);
    assert_eq!(stats.native_resources.explicit_closes, 0);
    assert_eq!(stats.native_resources.slot_reuses, 0);
    assert_eq!(stats.native_resources.cleanup_attempts, 0);
    assert_eq!(stats.native_resources.ordinary_obligations, 0);
    assert_eq!(stats.native_resources.borrowed_obligations, 0);
    assert_eq!(stats.native_resources.emergency_obligations, 0);
    assert_eq!(stats.native_resources.teardown_failures, 0);
    assert!(stats.code_objects.iter().all(|object| {
        object.runtime_calls.contains(&RuntimeCallSlot::StdinHandle)
            && !object
                .runtime_calls
                .contains(&RuntimeCallSlot::HeapDispatch)
            && object.wx_transition_verified
    }));
}
