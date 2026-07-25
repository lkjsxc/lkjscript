use super::super::*;

pub(crate) const fn evaluator_heap_object_bytes() -> usize {
    // Mirrors the deterministic base estimate of the current `HeapObj` enum
    // without coupling the dependency-free IR crate to the runtime crate.
    4 * std::mem::size_of::<usize>()
}

pub(crate) fn compare_values<T: PartialOrd>(
    operation: RuntimeOp,
    left: T,
    right: T,
) -> std::result::Result<bool, Flow> {
    match operation {
        RuntimeOp::Less => Ok(left < right),
        RuntimeOp::LessEqual => Ok(left <= right),
        RuntimeOp::Greater => Ok(left > right),
        RuntimeOp::GreaterEqual => Ok(left >= right),
        _ => Err(Flow::Trap("invalid comparison operation".into())),
    }
}

pub(crate) fn value_equal(left: &EvalValue, right: &EvalValue) -> std::result::Result<bool, Flow> {
    match (left, right) {
        (EvalValue::Unit, EvalValue::Unit) | (EvalValue::None, EvalValue::None) => Ok(true),
        (EvalValue::Bool(left), EvalValue::Bool(right)) => Ok(left == right),
        (EvalValue::I64(left), EvalValue::I64(right)) => Ok(left == right),
        (EvalValue::F64(left), EvalValue::F64(right)) => Ok(left == right),
        (EvalValue::Str(left), EvalValue::Str(right))
        | (EvalValue::Symbol(left), EvalValue::Symbol(right)) => Ok(left == right),
        (EvalValue::Some(left), EvalValue::Some(right))
        | (EvalValue::Ok(left), EvalValue::Ok(right))
        | (EvalValue::Err(left), EvalValue::Err(right)) => value_equal(left, right),
        (EvalValue::None, EvalValue::Some(_)) | (EvalValue::Some(_), EvalValue::None) => Ok(false),
        (EvalValue::Ok(_), EvalValue::Err(_)) | (EvalValue::Err(_), EvalValue::Ok(_)) => Ok(false),
        _ => Err(Flow::Trap("equal-value category mismatch".into())),
    }
}

pub(crate) fn unary<F>(
    arguments: &[EvalValue],
    operation: F,
) -> std::result::Result<EvalValue, Flow>
where
    F: FnOnce(&EvalValue) -> std::result::Result<EvalValue, Flow>,
{
    exact_arity(arguments, 1)?;
    let value = arguments
        .first()
        .ok_or_else(|| Flow::Trap("unary operand missing".into()))?;
    operation(value)
}

pub(crate) fn binary<F>(
    arguments: &[EvalValue],
    operation: F,
) -> std::result::Result<EvalValue, Flow>
where
    F: FnOnce(&EvalValue, &EvalValue) -> std::result::Result<EvalValue, Flow>,
{
    exact_arity(arguments, 2)?;
    let left = arguments
        .first()
        .ok_or_else(|| Flow::Trap("binary operand missing".into()))?;
    let right = arguments
        .get(1)
        .ok_or_else(|| Flow::Trap("binary operand missing".into()))?;
    operation(left, right)
}

pub(crate) fn ternary<F>(
    arguments: &[EvalValue],
    operation: F,
) -> std::result::Result<EvalValue, Flow>
where
    F: FnOnce(&EvalValue, &EvalValue, &EvalValue) -> std::result::Result<EvalValue, Flow>,
{
    exact_arity(arguments, 3)?;
    let first = arguments
        .first()
        .ok_or_else(|| Flow::Trap("ternary operand missing".into()))?;
    let second = arguments
        .get(1)
        .ok_or_else(|| Flow::Trap("ternary operand missing".into()))?;
    let third = arguments
        .get(2)
        .ok_or_else(|| Flow::Trap("ternary operand missing".into()))?;
    operation(first, second, third)
}

pub(crate) fn exact_arity(
    arguments: &[EvalValue],
    expected: usize,
) -> std::result::Result<(), Flow> {
    if arguments.len() == expected {
        Ok(())
    } else {
        Err(Flow::Trap("evaluator runtime arity mismatch".into()))
    }
}
