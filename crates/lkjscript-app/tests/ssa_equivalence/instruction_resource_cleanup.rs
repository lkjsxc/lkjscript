use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome};
use lkjscript_vm::run_chunk;

#[test]
fn instruction_failure_closes_live_vm_resource_without_emergency_teardown() {
    let path = format!(
        "/tmp/lkjscript-instruction-resource-cleanup-{}",
        std::process::id()
    );
    std::fs::write(&path, b"resource").expect("create instruction cleanup fixture");
    let source = format!(
        concat!(
            "main/\nsig/\ninputs/\ncapability/\nfile-system\n/capability\ncapability/\n",
            "stdio\n/capability\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n",
            "file-system\ncapability/\nfile-system\n/capability\nstdio\ncapability/\n",
            "stdio\n/capability\n/params\nlet/\nbind/\nreader\nunwrap-ok/\n",
            "open-file-reader/\nfile-system\nunwrap-ok/\nconvert-string-to-path/\n",
            "string-literal/\n{}\n/string-literal\n/convert-string-to-path\n/unwrap-ok\n",
            "/open-file-reader\n/unwrap-ok\n/bind\nunit\n/let\n/main\n"
        ),
        path
    );
    let repeated_reads = (0..4)
        .map(|_| {
            format!(
                "do/\n{}unit\n/do\n",
                "is-terminal/\nstandard-input/\nstdio\n/standard-input\n/is-terminal\n".repeat(6)
            )
        })
        .collect::<String>();
    let late_source = source.replacen(
        "unit\n/let\n/main\n",
        &format!("do/\n{repeated_reads}unit\n/do\n/let\n/main\n"),
        1,
    );
    let late = compile_source(&late_source, "instruction-resource-cleanup.lkjscript")
        .expect("compile instruction resource cleanup");
    let evaluator_config = EvalConfig {
        capabilities: vec![
            lkjscript_core::CapabilityKind::FileSystem,
            lkjscript_core::CapabilityKind::Stdio,
        ],
        ..EvalConfig::default()
    };
    let evaluator_full = evaluate(late.ssa(), &evaluator_config);
    assert!(
        matches!(evaluator_full, EvalOutcome::Returned(_)),
        "{evaluator_full:?}"
    );
    let evaluator_completion_fuel = (1..4_096_u64)
        .find(|fuel| {
            matches!(
                evaluate(
                    late.ssa(),
                    &EvalConfig {
                        fuel: *fuel,
                        ..evaluator_config.clone()
                    },
                ),
                EvalOutcome::Returned(_)
            )
        })
        .expect("bounded evaluator fuel completes resource fixture");
    let evaluator_interrupted = evaluate(
        late.ssa(),
        &EvalConfig {
            fuel: evaluator_completion_fuel / 2,
            ..evaluator_config
        },
    );
    assert!(
        evaluator_completion_fuel > 40,
        "{evaluator_completion_fuel}"
    );
    assert!(
        matches!(evaluator_interrupted, EvalOutcome::ResourceLimitExceeded(_))
            && evaluator_interrupted.cleanup_failures().is_none(),
        "{evaluator_interrupted:?}"
    );

    let completion_fuel = (1..512_u64)
        .find(|fuel| {
            matches!(
                run_chunk(
                    late.bytecode(),
                    &lkjscript_vm::ExecutionInputs {
                        arguments: Vec::new(),
                        capabilities: vec![
                            lkjscript_core::CapabilityKind::FileSystem,
                            lkjscript_core::CapabilityKind::Stdio,
                        ],
                        host: lkjscript_host::HostEnvironment::portable(),
                    },
                    &ExecutionConfig {
                        instruction_fuel: *fuel,
                        ..ExecutionConfig::default()
                    },
                ),
                ExecutionOutcome::Returned(_)
            )
        })
        .expect("bounded fuel completes resource fixture");
    let interrupted = run_chunk(
        late.bytecode(),
        &lkjscript_vm::ExecutionInputs {
            arguments: Vec::new(),
            capabilities: vec![
                lkjscript_core::CapabilityKind::FileSystem,
                lkjscript_core::CapabilityKind::Stdio,
            ],
            host: lkjscript_host::HostEnvironment::portable(),
        },
        &ExecutionConfig {
            instruction_fuel: completion_fuel / 2,
            ..ExecutionConfig::default()
        },
    );
    assert!(completion_fuel > 32, "{completion_fuel}");
    assert!(
        matches!(interrupted, ExecutionOutcome::ResourceLimitExceeded(_))
            && interrupted.cleanup_failures().is_none(),
        "{interrupted:?}"
    );

    let _removed = std::fs::remove_file(path);
}
