use crate::*;

#[derive(Debug)]
pub struct JitExecution {
    pub outcome: ExecutionOutcome,
    pub stats: JitStats,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarInvocationOutcome {
    Returned(NativeValue),
    Trapped(TrapCode, Option<u64>),
    Exited(i64),
    DeadlineExceeded,
    ResourceLimitExceeded(ResourceLimitKind),
    HostFailure,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarInvocation {
    pub outcome: ScalarInvocationOutcome,
    pub poll_count: u64,
    pub cleanup_failures: CleanupFailures,
}

pub(crate) fn scalar_to_execution(
    run: &mut NativeRun,
    function: FunctionId,
    outcome: ScalarInvocationOutcome,
) -> Result<ExecutionOutcome, EngineError> {
    Ok(match outcome {
        ScalarInvocationOutcome::Returned(value) => {
            let owned = match value {
                NativeValue::Reference(reference) => {
                    run.snapshot_reference_return(function, reference)?
                }
                NativeValue::StaticBytes(_) => run.take_returned_unique(function, true)?,
                NativeValue::Unique(owner)
                    if owner.unique_type() == lkjscript_native::UniqueType::ByteVector =>
                {
                    run.take_returned_unique(function, false)?
                }
                NativeValue::Unique(owner)
                    if owner.unique_type() == lkjscript_native::UniqueType::Bytes =>
                {
                    run.take_returned_unique(function, true)?
                }
                NativeValue::StructuralOwner(_) => {
                    let value = run.take_returned_structural(function)?;
                    OwnedValue::from_structural(value).map_err(|error| {
                        EngineError::new(
                            FailureCode::InvocationFailure,
                            Some(function),
                            error.to_string(),
                        )
                    })?
                }
                NativeValue::Capability(_)
                | NativeValue::Resource(_)
                | NativeValue::Unique(_)
                | NativeValue::Loan(_)
                | NativeValue::StructuralView(_)
                | NativeValue::StructuralDestination(_) => {
                    return Err(EngineError::new(
                        FailureCode::InvocationFailure,
                        Some(function),
                        "capability, resource, or loan escaped the native root",
                    ));
                }
                scalar => owned_scalar(scalar).map_err(|error| {
                    EngineError::new(
                        FailureCode::InvocationFailure,
                        Some(function),
                        error.to_string(),
                    )
                })?,
            };
            ExecutionOutcome::Returned(owned)
        }
        ScalarInvocationOutcome::Trapped(trap, site) => {
            ExecutionOutcome::Trapped(Trap::new(run.trap_message(function, trap, site)))
        }
        ScalarInvocationOutcome::Exited(code) => match i32::try_from(code) {
            Ok(code) => ExecutionOutcome::Exited(code),
            Err(_) => ExecutionOutcome::Trapped(Trap::new("exit code out of range")),
        },
        ScalarInvocationOutcome::DeadlineExceeded => ExecutionOutcome::DeadlineExceeded,
        ScalarInvocationOutcome::ResourceLimitExceeded(kind) => {
            ExecutionOutcome::ResourceLimitExceeded(kind)
        }
        ScalarInvocationOutcome::HostFailure => {
            ExecutionOutcome::HostFailure(HostError::new("native Poll host clock failure"))
        }
    })
}

pub(crate) fn owned_scalar(value: NativeValue) -> lkjscript_core::Result<OwnedValue> {
    match value {
        NativeValue::Unit => OwnedValue::from_value(Value::UNIT),
        NativeValue::Bool(value) => OwnedValue::from_value(Value::from_bool(value)),
        NativeValue::I64(value) => OwnedValue::from_value(Value::from_i64(value)),
        NativeValue::F64Bits(bits) => OwnedValue::from_value(Value::from_f64_bits(bits)),
        NativeValue::StaticBytes(_)
        | NativeValue::StaticString(_)
        | NativeValue::Capability(_)
        | NativeValue::Resource(_)
        | NativeValue::Unique(_)
        | NativeValue::Loan(_)
        | NativeValue::StructuralKey(_)
        | NativeValue::StructuralOwner(_)
        | NativeValue::StructuralView(_)
        | NativeValue::StructuralDestination(_)
        | NativeValue::Reference(_) => Err(lkjscript_core::Error::msg(
            "scalar JIT cannot return a native adapter value",
        )),
    }
}

pub fn native_type(ty: &SsaType) -> Option<ValueType> {
    match ty {
        SsaType::Unit => Some(ValueType::Unit),
        SsaType::Bool => Some(ValueType::Bool),
        SsaType::I64 => Some(ValueType::I64),
        SsaType::F64 => Some(ValueType::F64),
        _ => None,
    }
}
