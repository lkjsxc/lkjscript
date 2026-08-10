use std::time::{Duration, Instant};

use lkjscript_compiler::ExecutableProgram;
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_jit::{
    attempt_baseline_with_capabilities_from_start, BaselineAttempt, BaselineAttemptTimings,
    BaselineDeclineReason, JitConfig, JitStats,
};
use lkjscript_vm::{run_chunk_from_start, ExecutionInputs};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionPath {
    BaselineNative,
    VmFallback,
}

impl ExecutionPath {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BaselineNative => "baseline-native",
            Self::VmFallback => "vm-fallback",
        }
    }
}

pub struct Execution {
    pub outcome: ExecutionOutcome,
    pub stats: Option<JitStats>,
    pub path: ExecutionPath,
    pub decline: Option<BaselineDeclineReason>,
    pub native_entered: bool,
    pub native_timings: BaselineAttemptTimings,
    pub vm_duration: Duration,
    #[cfg(test)]
    pub vm_executions: u8,
}

pub fn execute(
    program: &ExecutableProgram,
    inputs: &ExecutionInputs,
    config: &ExecutionPolicy,
    jit_config: JitConfig,
    measure: bool,
) -> Result<Execution, String> {
    let execution_started = Instant::now();
    match attempt_baseline_with_capabilities_from_start(
        program.ssa(),
        &inputs.capabilities,
        config,
        jit_config,
        execution_started,
    ) {
        BaselineAttempt::Executed(execution) => Ok(Execution {
            outcome: execution.outcome,
            stats: Some(execution.stats),
            path: ExecutionPath::BaselineNative,
            decline: None,
            native_entered: true,
            native_timings: execution.timings,
            vm_duration: Duration::ZERO,
            #[cfg(test)]
            vm_executions: 0,
        }),
        BaselineAttempt::Declined(decline) => {
            // The one-shot API has already dropped all installed native state.
            // VM execution receives the original validated program, inputs, and
            // policy; no source effect or execution counter occurred pre-entry.
            let started = measure.then(Instant::now);
            let outcome =
                run_chunk_from_start(program.bytecode(), inputs, config, execution_started);
            let vm_duration = started.map_or(Duration::ZERO, |started| started.elapsed());
            Ok(Execution {
                outcome,
                stats: decline.stats,
                path: ExecutionPath::VmFallback,
                decline: Some(decline.reason),
                native_entered: false,
                native_timings: decline.timings,
                vm_duration,
                #[cfg(test)]
                vm_executions: 1,
            })
        }
        BaselineAttempt::EnteredFailure(failure) => Err(format!(
            "entered baseline-native execution failed without VM retry: {}",
            failure.error
        )),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::fmt::Write as _;
    use std::path::PathBuf;
    use std::sync::Arc;

    use lkjscript_compiler::{
        compile_path, compile_snapshot, compile_source,
        workspace::{
            DraftBindingId, DraftBindingRef, DraftNode, DraftNodeId, Edit, ExpressionDraft,
            LocalDraft, SemanticType, Transaction, Workspace,
        },
        Operation,
    };
    use lkjscript_core::{ExecutionOutcome, LimitedExecutionPolicy, ResourceLimitKind};

    use super::*;

    fn scalar(source: &str) -> ExecutableProgram {
        compile_source(source, "product-path-scalar.lkjscript").expect("compile scalar fixture")
    }

    fn scalar_main(expression: &str) -> ExecutableProgram {
        scalar(&format!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n{expression}\n/main\n"
        ))
    }

    fn source_free_ownership_control() -> ExecutableProgram {
        let mut workspace = Workspace::empty().expect("empty source-free workspace");
        let created = workspace
            .apply(Transaction {
                base_revision: workspace.current().revision(),
                edits: vec![Edit::CreateMain {
                    return_type: SemanticType::I64,
                }],
            })
            .expect("create source-free main");
        let hole = created.snapshot.holes().next().expect("main hole").id;
        let owner = DraftBindingId::new(0);
        let completed = workspace
            .apply(Transaction {
                base_revision: created.snapshot.revision(),
                edits: vec![Edit::FillHole {
                    hole,
                    draft: ExpressionDraft::new(
                        vec![
                            DraftNode::I64(2),
                            DraftNode::Operation {
                                operation: Operation::ByteVectorNew,
                                arguments: vec![DraftNodeId::new(0)],
                            },
                            DraftNode::BorrowShared(DraftBindingRef::Local(owner)),
                            DraftNode::Operation {
                                operation: Operation::ByteSliceLength,
                                arguments: vec![DraftNodeId::new(2)],
                            },
                            DraftNode::I64(7),
                            DraftNode::Return {
                                value: DraftNodeId::new(4),
                            },
                            DraftNode::Sequence(vec![DraftNodeId::new(3), DraftNodeId::new(5)]),
                            DraftNode::Let {
                                bindings: vec![LocalDraft {
                                    binding: owner,
                                    name: "b".to_owned(),
                                    value: DraftNodeId::new(1),
                                }],
                                body: DraftNodeId::new(6),
                            },
                        ],
                        DraftNodeId::new(7),
                    ),
                }],
            })
            .expect("construct source-free ownership-control");
        compile_snapshot(&completed.snapshot).expect("compile source-free ownership-control")
    }

    fn direct_call_chain(functions: usize) -> ExecutableProgram {
        let mut source = String::new();
        for index in 0..functions {
            let body = if index + 1 == functions {
                "42".to_string()
            } else {
                format!("chain-{}/\n/chain-{}", index + 1, index + 1)
            };
            write!(
                source,
                "def/\nname/\nchain-{index}\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\n/params\n{body}\n/fn\n/def\n"
            )
            .expect("write direct-call chain source");
        }
        source.push_str(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nchain-0/\n/chain-0\n/main\n",
        );
        scalar(&source)
    }

    #[test]
    fn scalar_group_executes_once_in_baseline_native() {
        let program = scalar_main("add/\n40\n2\n/add");
        let execution = execute(
            &program,
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
            true,
        )
        .expect("execute product path");
        assert_eq!(execution.path, ExecutionPath::BaselineNative);
        assert!(execution.native_entered);
        assert_eq!(execution.vm_executions, 0);
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::Returned(value) if value.as_i64() == Some(42)
        ));
    }

    #[test]
    fn one_shot_baseline_executes_direct_generated_calls() {
        let program = scalar(concat!(
            "def/\nname/\nadd-one\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\n",
            "output/\ni64\n/output\n/sig\nparams/\nvalue\ni64\n/params\n",
            "add/\nvalue\n1\n/add\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\n",
            "output/\ni64\n/output\n/sig\nadd-one/\n41\n/add-one\n/main\n",
        ));
        let execution = execute(
            &program,
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
            false,
        )
        .expect("execute direct-call group");
        assert_eq!(execution.path, ExecutionPath::BaselineNative);
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::Returned(value) if value.as_i64() == Some(42)
        ));
        assert!(execution
            .stats
            .as_ref()
            .is_some_and(|stats| stats.direct_native_calls > 0));
    }

    #[test]
    fn one_shot_baseline_executes_structural_unique_and_resource_islands() {
        let structural = scalar(concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\nstring\n/output\n/sig\n",
            "empty-string/\n/empty-string\n/main\n",
        ));
        let structural = execute(
            &structural,
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
            false,
        )
        .expect("execute structural island");
        assert_eq!(structural.path, ExecutionPath::BaselineNative);
        assert!(
            matches!(
                structural.outcome,
                ExecutionOutcome::Returned(ref value) if value.as_str() == Some("")
            ),
            "{:?}",
            structural.outcome
        );
        assert!(structural
            .stats
            .as_ref()
            .is_some_and(|stats| stats.structural_runtime_calls > 0));

        let unique = scalar(concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
            "let/\nbind/\nb\nnew-byte-vector/\n3\n/new-byte-vector\n/bind\n",
            "byte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/let\n/main\n",
        ));
        let unique = execute(
            &unique,
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
            false,
        )
        .expect("execute unique island");
        assert_eq!(unique.path, ExecutionPath::BaselineNative);
        assert!(matches!(
            unique.outcome,
            ExecutionOutcome::Returned(value) if value.as_i64() == Some(3)
        ));
        assert!(unique
            .stats
            .as_ref()
            .is_some_and(|stats| stats.unique_runtime_calls > 0));

        let resource = scalar(concat!(
            "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\n",
            "output/\nunit\n/output\n/sig\nparams/\nstdio\ncapability/\nstdio\n",
            "/capability\n/params\ndo/\nstandard-input/\nstdio\n/standard-input\n",
            "standard-input/\nstdio\n/standard-input\nunit\n/do\n/main\n",
        ));
        let resource = execute(
            &resource,
            &ExecutionInputs {
                arguments: Vec::new(),
                capabilities: vec![lkjscript_core::CapabilityKind::Stdio],
                host: lkjscript_host::HostEnvironment::portable(),
            },
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
            false,
        )
        .expect("execute resource island");
        assert_eq!(resource.path, ExecutionPath::BaselineNative);
        assert!(matches!(
            resource.outcome,
            ExecutionOutcome::Returned(value) if value.is_unit()
        ));
        assert!(resource
            .stats
            .as_ref()
            .is_some_and(|stats| stats.resource_runtime_calls == 2));
    }

    #[test]
    fn source_free_early_return_enters_native_and_cleans_unique_owner_once() {
        let imported = scalar(include_str!(
            "../tests/fixtures/ownership-control.lkjscript"
        ));
        let source_free = source_free_ownership_control();
        let mut observations = Vec::new();
        for program in [&imported, &source_free] {
            let execution = execute(
                program,
                &ExecutionInputs::default(),
                &ExecutionPolicy::unrestricted(),
                JitConfig::default(),
                false,
            )
            .expect("execute ownership-control through selected product path");
            assert_eq!(execution.path, ExecutionPath::BaselineNative);
            assert!(execution.native_entered);
            assert_eq!(execution.vm_executions, 0);
            assert!(execution.outcome.cleanup_failures().is_none());
            assert!(matches!(
                execution.outcome,
                ExecutionOutcome::Returned(value) if value.as_i64() == Some(7)
            ));
            let unique = execution.stats.expect("native stats").native_unique;
            assert_eq!(unique.allocations, 1);
            assert_eq!(unique.drops, 1);
            assert_eq!(unique.live_owners, 0);
            assert_eq!(unique.live_loans, 0);
            assert_eq!(unique.release_backlog, 0);
            assert_eq!(unique.stale_or_forged_failures, 0);
            assert_eq!(unique.teardown_failures, 0);
            observations.push(unique);
        }
        assert_eq!(observations[0], observations[1]);
    }

    #[test]
    fn generated_group_crosses_backend_function_admission_and_falls_back_once() {
        let program = direct_call_chain(64);
        let execution = execute(
            &program,
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
            true,
        )
        .expect("wide native group falls back to the VM");
        assert_eq!(execution.path, ExecutionPath::VmFallback);
        assert!(!execution.native_entered);
        assert_eq!(execution.vm_executions, 1);
        let decline = execution.decline.expect("typed native decline");
        assert_eq!(decline.stage(), "lowering");
        assert_eq!(decline.code(), "backend-verification");
        assert!(decline.detail().contains("function count"));
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::Returned(value) if value.as_i64() == Some(42)
        ));
    }

    #[test]
    fn installation_decline_publishes_no_artifact_and_falls_back_once() {
        let program = scalar_main("add/\n40\n2\n/add");
        let config = JitConfig {
            retain_machine_code_diagnostics: true,
            max_diagnostic_bytes: 0,
            ..JitConfig::default()
        };
        let execution = execute(
            &program,
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
            config,
            true,
        )
        .expect("installation decline falls back");
        assert_eq!(execution.path, ExecutionPath::VmFallback);
        assert!(!execution.native_entered);
        assert_eq!(execution.vm_executions, 1);
        let decline = execution.decline.expect("typed installation decline");
        assert_eq!(decline.stage(), "installation");
        assert_eq!(decline.code(), "install-limit");
        assert!(execution
            .stats
            .as_ref()
            .is_some_and(|stats| stats.code_objects.is_empty()));
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::Returned(value) if value.as_i64() == Some(42)
        ));
    }

    #[test]
    fn typed_pre_entry_decline_runs_the_vm_once_with_the_original_policy() {
        let program = scalar_main("add/\n40\n2\n/add");
        let policy = ExecutionPolicy::limited(LimitedExecutionPolicy {
            instruction_fuel: 0,
            ..LimitedExecutionPolicy::conservative()
        });
        let execution = execute(
            &program,
            &ExecutionInputs::default(),
            &policy,
            JitConfig::default(),
            true,
        )
        .expect("pre-entry decline falls back");
        assert_eq!(execution.path, ExecutionPath::VmFallback);
        assert!(!execution.native_entered);
        assert_eq!(execution.vm_executions, 1);
        assert_eq!(
            execution.decline.as_ref().map(BaselineDeclineReason::stage),
            Some("preparation")
        );
        assert_eq!(
            execution.decline.as_ref().map(BaselineDeclineReason::code),
            Some("resource-limit-exceeded")
        );
        assert_eq!(
            execution.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::InstructionFuel)
        );
    }

    #[test]
    fn pre_entry_decline_and_vm_share_one_wall_deadline() {
        let program = scalar_main("add/\n40\n2\n/add");
        let policy = ExecutionPolicy::limited(LimitedExecutionPolicy {
            wall_time: Some(Duration::from_millis(1)),
            ..LimitedExecutionPolicy::conservative()
        });
        let started = Instant::now()
            .checked_sub(Duration::from_millis(10))
            .expect("past monotonic instant");
        let attempt = attempt_baseline_with_capabilities_from_start(
            program.ssa(),
            &[],
            &policy,
            JitConfig::default(),
            started,
        );
        assert!(matches!(
            attempt,
            BaselineAttempt::Declined(lkjscript_jit::BaselineDecline {
                reason: lkjscript_jit::BaselineDeclineReason::Preparation(
                    lkjscript_jit::PreEntryError::DeadlineExceeded
                ),
                ..
            })
        ));
        assert_eq!(
            run_chunk_from_start(
                program.bytecode(),
                &ExecutionInputs::default(),
                &policy,
                started,
            ),
            ExecutionOutcome::DeadlineExceeded
        );
    }

    #[test]
    fn entered_trap_never_reexecutes_in_the_vm() {
        let program = scalar_main("divide/\n1\n0\n/divide");
        let execution = execute(
            &program,
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
            true,
        )
        .expect("entered trap is an execution outcome");
        assert_eq!(execution.path, ExecutionPath::BaselineNative);
        assert!(execution.native_entered);
        assert_eq!(execution.vm_executions, 0);
        assert!(matches!(execution.outcome, ExecutionOutcome::Trapped(_)));
    }

    #[test]
    fn unsupported_examples_fallback_and_observable_output_occurs_once() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for relative in [
            "src/examples/hello/main.lkjscript",
            "src/examples/bench/main.lkjscript",
            "src/examples/mandel/main.lkjscript",
        ] {
            let program = compile_path(&root.join(relative)).expect("compile fallback example");
            let stdio = lkjscript_host::BufferedStdio::default();
            let inputs = ExecutionInputs {
                arguments: Vec::new(),
                capabilities: program.bytecode().required_capabilities().to_vec(),
                host: lkjscript_host::HostEnvironment {
                    stdio: Some(Arc::new(stdio.clone())),
                    ..lkjscript_host::HostEnvironment::default()
                },
            };
            let execution = execute(
                &program,
                &inputs,
                &ExecutionPolicy::unrestricted(),
                JitConfig::default(),
                false,
            )
            .expect("unsupported group uses VM");
            assert_eq!(execution.path, ExecutionPath::VmFallback, "{relative}");
            assert!(!execution.native_entered, "{relative}");
            assert_eq!(execution.vm_executions, 1, "{relative}");
            assert!(
                !stdio.output().expect("captured output").is_empty(),
                "{relative}"
            );
        }

        let observable = scalar(concat!(
            "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\n",
            "output/\nunit\n/output\n/sig\nparams/\nstdio\ncapability/\nstdio\n/capability\n/params\n",
            "do/\nprint/\nstdio\nstring-literal/\nonce\n/string-literal\n/print\n",
            "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\n",
            "unit\n/let\n/do\n/main\n",
        ));
        let stdio = lkjscript_host::BufferedStdio::default();
        let inputs = ExecutionInputs {
            arguments: Vec::new(),
            capabilities: observable.bytecode().required_capabilities().to_vec(),
            host: lkjscript_host::HostEnvironment {
                stdio: Some(Arc::new(stdio.clone())),
                ..lkjscript_host::HostEnvironment::default()
            },
        };
        let execution = execute(
            &observable,
            &inputs,
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
            false,
        )
        .expect("observable unsupported group uses VM");
        assert_eq!(execution.vm_executions, 1);
        assert_eq!(stdio.output().expect("captured output"), b"once");
    }
}
