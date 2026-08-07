use std::time::{Duration, Instant};

use lkjscript_compiler::ExecutableProgram;
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_jit::{
    attempt_baseline_with_capabilities_from_start, BaselineAttempt, BaselineAttemptTimings,
    JitConfig, JitStats,
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
    pub fallback_reason: Option<&'static str>,
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
        program.bytecode_links(),
        &inputs.capabilities,
        config,
        jit_config,
        execution_started,
    ) {
        BaselineAttempt::Executed(execution) => Ok(Execution {
            outcome: execution.outcome,
            stats: Some(execution.stats),
            path: ExecutionPath::BaselineNative,
            fallback_reason: None,
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
                fallback_reason: Some(decline.reason.metric_label()),
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
    use std::path::PathBuf;
    use std::sync::Arc;

    use lkjscript_compiler::{compile_path, compile_source};
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
        assert_eq!(execution.fallback_reason, Some("pre-entry-declined"));
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
            program.bytecode_links(),
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
