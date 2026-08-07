use std::fmt;

use crate::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BaselineAttemptTimings {
    pub preflight: Duration,
    pub lowering_and_encoding: Duration,
    pub installation: Duration,
    pub preparation: Duration,
    pub native_execution: Duration,
}

#[derive(Debug)]
pub struct BaselineExecution {
    pub outcome: ExecutionOutcome,
    pub stats: JitStats,
    pub timings: BaselineAttemptTimings,
}

#[derive(Debug)]
pub struct BaselineDecline {
    pub reason: BaselineDeclineReason,
    pub stats: Option<JitStats>,
    pub timings: BaselineAttemptTimings,
}

#[derive(Debug)]
pub struct BaselineEnteredFailure {
    pub error: BaselineEnteredError,
    pub stats: JitStats,
    pub timings: BaselineAttemptTimings,
}

#[derive(Debug)]
pub enum BaselineAttempt {
    Executed(BaselineExecution),
    Declined(BaselineDecline),
    EnteredFailure(BaselineEnteredFailure),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BaselineDeclineReason {
    Lowering(EngineError),
    Installation(EngineError),
    Preparation(PreEntryError),
    PreparationFailure(EngineError),
}

impl BaselineDeclineReason {
    pub const fn metric_label(&self) -> &'static str {
        match self {
            Self::Lowering(_) => "lowering-declined",
            Self::Installation(_) => "installation-declined",
            Self::Preparation(_) => "pre-entry-declined",
            Self::PreparationFailure(_) => "preparation-failed",
        }
    }
}

impl fmt::Display for BaselineDeclineReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lowering(error) => write!(formatter, "lowering declined: {error}"),
            Self::Installation(error) => write!(formatter, "installation declined: {error}"),
            Self::Preparation(error) => write!(formatter, "pre-entry declined: {error}"),
            Self::PreparationFailure(error) => write!(formatter, "preparation failed: {error}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BaselineEnteredError {
    Invocation(EnteredInvocationError),
    Completion(EngineError),
}

impl fmt::Display for BaselineEnteredError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invocation(error) => {
                write!(formatter, "entered native invocation failed: {error}")
            }
            Self::Completion(error) => {
                write!(formatter, "entered native completion failed: {error}")
            }
        }
    }
}

impl std::error::Error for BaselineEnteredError {}

pub(crate) enum BaselineRegionAttempt {
    Declined {
        error: PreEntryError,
        preparation: Duration,
    },
    PreparationFailure {
        error: EngineError,
        preparation: Duration,
    },
    Entered {
        result: Box<Result<InvocationReport, EnteredInvocationError>>,
        preparation: Duration,
        native_execution: Duration,
    },
}

pub(crate) enum BaselineScalarAttempt {
    Executed {
        invocation: ScalarInvocation,
        preparation: Duration,
        native_execution: Duration,
    },
    Declined {
        error: PreEntryError,
        preparation: Duration,
    },
    PreparationFailure {
        error: EngineError,
        preparation: Duration,
    },
    EnteredFailure {
        error: EnteredInvocationError,
        preparation: Duration,
        native_execution: Duration,
    },
}

pub fn attempt_baseline(
    program: &VerifiedProgram,
    execution: &ExecutionPolicy,
    config: JitConfig,
) -> BaselineAttempt {
    attempt_baseline_with_capabilities_from_start(program, &[], execution, config, Instant::now())
}

pub fn attempt_baseline_with_capabilities(
    program: &VerifiedProgram,
    capabilities: &[lkjscript_core::CapabilityKind],
    execution: &ExecutionPolicy,
    config: JitConfig,
) -> BaselineAttempt {
    attempt_baseline_with_capabilities_from_start(
        program,
        capabilities,
        execution,
        config,
        Instant::now(),
    )
}

pub fn attempt_baseline_with_capabilities_from_start(
    program: &VerifiedProgram,
    capabilities: &[lkjscript_core::CapabilityKind],
    execution: &ExecutionPolicy,
    config: JitConfig,
    execution_started: Instant,
) -> BaselineAttempt {
    let mut timings = BaselineAttemptTimings::default();
    let preflight_started = Instant::now();
    let arguments = match capability_arguments(program, capabilities) {
        Ok(arguments) => arguments,
        Err(error) => {
            timings.preflight = preflight_started.elapsed();
            return BaselineAttempt::Declined(BaselineDecline {
                reason: BaselineDeclineReason::PreparationFailure(error),
                stats: None,
                timings,
            });
        }
    };
    timings.preflight = preflight_started.elapsed();

    let main = program.program().main;
    let mut run = NativeRun::new_baseline_attempt(program, config);
    if let Err(error) = run.compile_group(main) {
        timings.lowering_and_encoding = run.last_lowering_and_encoding;
        timings.installation = run.last_installation;
        let reason = match error.code() {
            FailureCode::InstallLimit | FailureCode::InstallFailure => {
                BaselineDeclineReason::Installation(error)
            }
            _ => BaselineDeclineReason::Lowering(error),
        };
        return BaselineAttempt::Declined(BaselineDecline {
            reason,
            stats: Some(run.stats()),
            timings,
        });
    }
    timings.lowering_and_encoding = run.last_lowering_and_encoding;
    timings.installation = run.last_installation;

    let invocation_policy = remaining_execution_policy(execution, execution_started);
    match run.invoke_baseline_scalar_attempt(main, &arguments, &invocation_policy) {
        BaselineScalarAttempt::Executed {
            invocation,
            preparation,
            native_execution,
        } => {
            timings.preparation = preparation;
            timings.native_execution = native_execution;
            let outcome = scalar_to_execution(&mut run, main, invocation.outcome)
                .map(|outcome| outcome.with_cleanup_failures(invocation.cleanup_failures));
            match outcome {
                Ok(outcome) => BaselineAttempt::Executed(BaselineExecution {
                    outcome,
                    stats: run.stats(),
                    timings,
                }),
                Err(error) => BaselineAttempt::EnteredFailure(BaselineEnteredFailure {
                    error: BaselineEnteredError::Completion(error),
                    stats: run.stats(),
                    timings,
                }),
            }
        }
        BaselineScalarAttempt::Declined { error, preparation } => {
            timings.preparation = preparation;
            BaselineAttempt::Declined(BaselineDecline {
                reason: BaselineDeclineReason::Preparation(error),
                stats: Some(run.stats()),
                timings,
            })
        }
        BaselineScalarAttempt::PreparationFailure { error, preparation } => {
            timings.preparation = preparation;
            BaselineAttempt::Declined(BaselineDecline {
                reason: BaselineDeclineReason::PreparationFailure(error),
                stats: Some(run.stats()),
                timings,
            })
        }
        BaselineScalarAttempt::EnteredFailure {
            error,
            preparation,
            native_execution,
        } => {
            timings.preparation = preparation;
            timings.native_execution = native_execution;
            BaselineAttempt::EnteredFailure(BaselineEnteredFailure {
                error: BaselineEnteredError::Invocation(error),
                stats: run.stats(),
                timings,
            })
        }
    }
}

fn remaining_execution_policy(
    execution: &ExecutionPolicy,
    execution_started: Instant,
) -> ExecutionPolicy {
    let mut remaining = execution.clone();
    if let Some(policy) = remaining.limited_policy_mut() {
        if let Some(wall_time) = policy.wall_time {
            policy.wall_time = Some(
                wall_time
                    .checked_sub(execution_started.elapsed())
                    .unwrap_or(Duration::ZERO),
            );
        }
    }
    remaining
}
