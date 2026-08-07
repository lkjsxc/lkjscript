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
    UnsupportedShape(String),
    Lowering(EngineError),
    Installation(EngineError),
    Preparation(PreEntryError),
    PreparationFailure(EngineError),
}

impl BaselineDeclineReason {
    pub const fn metric_label(&self) -> &'static str {
        match self {
            Self::UnsupportedShape(_) => "unsupported-shape",
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
            Self::UnsupportedShape(detail) => write!(formatter, "unsupported shape: {detail}"),
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
    links: &BytecodeLinkMetadata,
    execution: &ExecutionPolicy,
    config: JitConfig,
) -> BaselineAttempt {
    attempt_baseline_with_capabilities_from_start(
        program,
        links,
        &[],
        execution,
        config,
        Instant::now(),
    )
}

pub fn attempt_baseline_with_capabilities(
    program: &VerifiedProgram,
    links: &BytecodeLinkMetadata,
    capabilities: &[lkjscript_core::CapabilityKind],
    execution: &ExecutionPolicy,
    config: JitConfig,
) -> BaselineAttempt {
    attempt_baseline_with_capabilities_from_start(
        program,
        links,
        capabilities,
        execution,
        config,
        Instant::now(),
    )
}

pub fn attempt_baseline_with_capabilities_from_start(
    program: &VerifiedProgram,
    links: &BytecodeLinkMetadata,
    capabilities: &[lkjscript_core::CapabilityKind],
    execution: &ExecutionPolicy,
    config: JitConfig,
    execution_started: Instant,
) -> BaselineAttempt {
    let mut timings = BaselineAttemptTimings::default();
    let preflight_started = Instant::now();
    let eligibility = preflight_product_group(program, capabilities);
    timings.preflight = preflight_started.elapsed();
    if let Err(detail) = eligibility {
        return BaselineAttempt::Declined(BaselineDecline {
            reason: BaselineDeclineReason::UnsupportedShape(detail),
            stats: None,
            timings,
        });
    }

    let main = program.program().main;
    let mut session = JitSession::new_baseline_attempt(program, links, config);
    if let Err(error) = session.compile_group(main) {
        timings.lowering_and_encoding = session.last_lowering_and_encoding;
        timings.installation = session.last_installation;
        let reason = match error.code() {
            FailureCode::InstallLimit | FailureCode::InstallFailure => {
                BaselineDeclineReason::Installation(error)
            }
            _ => BaselineDeclineReason::Lowering(error),
        };
        return BaselineAttempt::Declined(BaselineDecline {
            reason,
            stats: Some(session.stats()),
            timings,
        });
    }
    timings.lowering_and_encoding = session.last_lowering_and_encoding;
    timings.installation = session.last_installation;

    let arguments = match capability_arguments(program, capabilities) {
        Ok(arguments) => arguments,
        Err(error) => {
            return BaselineAttempt::Declined(BaselineDecline {
                reason: BaselineDeclineReason::PreparationFailure(error),
                stats: Some(session.stats()),
                timings,
            });
        }
    };
    let invocation_policy = remaining_execution_policy(execution, execution_started);
    match session.invoke_baseline_scalar_attempt(main, &arguments, &invocation_policy) {
        BaselineScalarAttempt::Executed {
            invocation,
            preparation,
            native_execution,
        } => {
            timings.preparation = preparation;
            timings.native_execution = native_execution;
            let outcome = scalar_to_execution(&mut session, main, invocation.outcome)
                .map(|outcome| outcome.with_cleanup_failures(invocation.cleanup_failures));
            match outcome {
                Ok(outcome) => BaselineAttempt::Executed(BaselineExecution {
                    outcome,
                    stats: session.stats(),
                    timings,
                }),
                Err(error) => BaselineAttempt::EnteredFailure(BaselineEnteredFailure {
                    error: BaselineEnteredError::Completion(error),
                    stats: session.stats(),
                    timings,
                }),
            }
        }
        BaselineScalarAttempt::Declined { error, preparation } => {
            timings.preparation = preparation;
            BaselineAttempt::Declined(BaselineDecline {
                reason: BaselineDeclineReason::Preparation(error),
                stats: Some(session.stats()),
                timings,
            })
        }
        BaselineScalarAttempt::PreparationFailure { error, preparation } => {
            timings.preparation = preparation;
            BaselineAttempt::Declined(BaselineDecline {
                reason: BaselineDeclineReason::PreparationFailure(error),
                stats: Some(session.stats()),
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
                stats: session.stats(),
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

fn preflight_product_group(
    program: &VerifiedProgram,
    capabilities: &[lkjscript_core::CapabilityKind],
) -> Result<(), String> {
    let program = program.program();
    if !capabilities.is_empty() {
        return Err("capability-bearing main requires the generic VM".to_string());
    }
    let functions =
        lower::reachable_group(program, program.main).map_err(|error| error.to_string())?;
    for id in functions {
        let index = id.index().ok_or_else(|| {
            "reachable function identity is outside host representation".to_string()
        })?;
        let function = program
            .functions
            .get(index)
            .filter(|function| function.id == id)
            .ok_or_else(|| "reachable function is absent from verified SSA".to_string())?;
        if id == program.main && !function.signature.parameters.is_empty() {
            return Err("scalar native main does not accept source arguments".to_string());
        }
        if function.signature.parameters.len() > 2
            || !function.signature.type_parameters.is_empty()
            || !function.signature.bounds.is_empty()
            || !function.signature.memory_witness_parameters.is_empty()
            || !function
                .signature
                .parameters
                .iter()
                .chain(std::iter::once(function.signature.result.as_ref()))
                .all(scalar_type)
        {
            return Err(format!(
                "function {} has a non-scalar or generic native signature",
                id.raw()
            ));
        }
        for block in &function.blocks {
            if !block
                .parameters
                .iter()
                .all(|parameter| scalar_type(&parameter.ty))
            {
                return Err(format!(
                    "function {} has a non-scalar block parameter",
                    id.raw()
                ));
            }
            for instruction in &block.instructions {
                if !scalar_type(&instruction.ty) || !scalar_instruction(&instruction.kind) {
                    return Err(format!(
                        "function {} reaches a structural, I/O, or generic operation",
                        id.raw()
                    ));
                }
            }
        }
    }
    Ok(())
}

fn scalar_type(ty: &SsaType) -> bool {
    matches!(
        ty,
        SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
    )
}

fn scalar_instruction(kind: &lkjscript_ir::InstructionKind) -> bool {
    use lkjscript_ir::InstructionKind;
    match kind {
        InstructionKind::Constant(_) | InstructionKind::Copy(_) => true,
        InstructionKind::Runtime { operation, .. } => matches!(
            operation,
            lkjscript_ir::RuntimeOp::Add
                | lkjscript_ir::RuntimeOp::Subtract
                | lkjscript_ir::RuntimeOp::Multiply
                | lkjscript_ir::RuntimeOp::Divide
                | lkjscript_ir::RuntimeOp::EqualValue
                | lkjscript_ir::RuntimeOp::SameObject
                | lkjscript_ir::RuntimeOp::F64BitsEqual
                | lkjscript_ir::RuntimeOp::Less
                | lkjscript_ir::RuntimeOp::LessEqual
                | lkjscript_ir::RuntimeOp::Greater
                | lkjscript_ir::RuntimeOp::GreaterEqual
                | lkjscript_ir::RuntimeOp::Not
                | lkjscript_ir::RuntimeOp::BitAnd
                | lkjscript_ir::RuntimeOp::BitOr
                | lkjscript_ir::RuntimeOp::BitXor
        ),
        InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::F64FromI64Rounded { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. } => true,
        InstructionKind::Call {
            target: lkjscript_ir::CallTarget::Direct(_),
            instantiation: None,
            ..
        } => true,
        _ => false,
    }
}
