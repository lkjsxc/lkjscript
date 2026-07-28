use crate::*;

#[derive(Debug)]
pub struct JitExecution {
    pub outcome: ExecutionOutcome,
    pub stats: JitStats,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarSignature {
    pub(crate) parameters: Vec<ValueType>,
    pub(crate) result: ValueType,
}

impl ScalarSignature {
    pub fn parameters(&self) -> &[ValueType] {
        &self.parameters
    }

    pub const fn result(&self) -> ValueType {
        self.result
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryDecision {
    Interpret,
    Native(FunctionId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarInvocationOutcome {
    Returned(NativeValue),
    Trapped(TrapCode, Option<u32>),
    Exited(i64),
    DeadlineExceeded,
    ResourceLimitExceeded(ResourceLimitKind),
    HostFailure,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarInvocation {
    pub outcome: ScalarInvocationOutcome,
    pub poll_count: u64,
}

pub(crate) fn scalar_to_execution(
    session: &JitSession,
    function: FunctionId,
    outcome: ScalarInvocationOutcome,
) -> Result<ExecutionOutcome, EngineError> {
    Ok(match outcome {
        ScalarInvocationOutcome::Returned(value) => {
            let owned = match value {
                NativeValue::Reference(reference) => {
                    let value =
                        native_reference_value(&session.heap, reference).map_err(|error| {
                            EngineError::new(FailureCode::InvocationFailure, Some(function), error)
                        })?;
                    session.heap.snapshot(value).map_err(|error| {
                        EngineError::new(
                            FailureCode::InvocationFailure,
                            Some(function),
                            error.to_string(),
                        )
                    })?
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
            ExecutionOutcome::Trapped(Trap::new(session.trap_message(function, trap, site)))
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
        NativeValue::Unit => OwnedValue::from_vm_snapshot(Value::UNIT, Vec::new()),
        NativeValue::Bool(value) => {
            OwnedValue::from_vm_snapshot(Value::from_bool(value), Vec::new())
        }
        NativeValue::I64(value) => OwnedValue::from_vm_snapshot(Value::from_i64(value), Vec::new()),
        NativeValue::F64Bits(bits) => {
            OwnedValue::from_vm_snapshot(Value::from_f64_bits(bits), Vec::new())
        }
        NativeValue::Reference(_) => Err(lkjscript_core::Error::msg(
            "scalar JIT cannot return a native reference",
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn detached_native_scalars_retain_exact_payload_without_snapshot_objects() {
        for value in [i64::MIN, -1, 0, 1, i64::MAX] {
            let owned = owned_scalar(NativeValue::I64(value)).expect("owned I64");
            assert_eq!(owned.as_i64(), Some(value));
            assert_eq!(owned.snapshot_object_count(), 0);
        }
        for bits in [
            0_u64,
            1_u64 << 63,
            0x7ff0_0000_0000_0000,
            0x7ff8_0000_0000_0042,
            0xfff8_dead_beef_cafe,
        ] {
            let owned = owned_scalar(NativeValue::F64Bits(bits)).expect("owned F64");
            assert_eq!(owned.as_f64_bits(), Some(bits));
            assert_eq!(owned.snapshot_object_count(), 0);
        }
    }
}
