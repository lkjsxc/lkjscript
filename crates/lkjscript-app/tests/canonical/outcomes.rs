use lkjscript_core::{ExecutionOutcome, OwnedValue, ResourceLimitKind};
use lkjscript_ir::{EvalOutcome, EvalValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scalar {
    Unit,
    Bool(bool),
    I64(i64),
    F64(u64),
    Exited(i64),
    Trapped,
    Deadline,
    Fuel,
    Other(String),
}

fn owned(value: &OwnedValue) -> Scalar {
    if value.is_unit() {
        Scalar::Unit
    } else if let Some(value) = value.as_bool() {
        Scalar::Bool(value)
    } else if let Some(value) = value.as_i64() {
        Scalar::I64(value)
    } else if let Some(value) = value.as_f64() {
        Scalar::F64(value.to_bits())
    } else {
        Scalar::Other(format!("{value:?}"))
    }
}

pub fn execution(outcome: ExecutionOutcome) -> Scalar {
    match outcome {
        ExecutionOutcome::Returned(value) => owned(&value),
        ExecutionOutcome::Exited(code) => Scalar::Exited(i64::from(code)),
        ExecutionOutcome::Trapped(_) => Scalar::Trapped,
        ExecutionOutcome::DeadlineExceeded => Scalar::Deadline,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::InstructionFuel) => Scalar::Fuel,
        other => Scalar::Other(other.summary()),
    }
}

pub fn evaluator(outcome: EvalOutcome) -> Scalar {
    match outcome {
        EvalOutcome::Returned(EvalValue::Unit) => Scalar::Unit,
        EvalOutcome::Returned(EvalValue::Bool(value)) => Scalar::Bool(value),
        EvalOutcome::Returned(EvalValue::I64(value)) => Scalar::I64(value),
        EvalOutcome::Returned(EvalValue::F64(value)) => Scalar::F64(value.to_bits()),
        EvalOutcome::Exited(code) => Scalar::Exited(code),
        EvalOutcome::Trapped(_) => Scalar::Trapped,
        other => Scalar::Other(format!("{other:?}")),
    }
}
