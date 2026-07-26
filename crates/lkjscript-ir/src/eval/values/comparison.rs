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
        (EvalValue::Unit, EvalValue::Unit) => Ok(true),
        (EvalValue::Bool(left), EvalValue::Bool(right)) => Ok(left == right),
        (EvalValue::I64(left), EvalValue::I64(right)) => Ok(left == right),
        (EvalValue::F64(left), EvalValue::F64(right)) => Ok(left == right),
        (EvalValue::Str(left), EvalValue::Str(right))
        | (EvalValue::Symbol(left), EvalValue::Symbol(right)) => Ok(left == right),
        (EvalValue::Path(left), EvalValue::Path(right)) => Ok(left == right),
        (
            EvalValue::Enum {
                enum_id: left_enum,
                variant: left_variant,
                ..
            },
            EvalValue::Enum {
                enum_id: right_enum,
                variant: right_variant,
                ..
            },
        ) if left_enum == right_enum && left_variant != right_variant => Ok(false),
        (
            EvalValue::Enum {
                enum_id: left_enum,
                variant: left_variant,
                payload: left_payload,
                ..
            },
            EvalValue::Enum {
                enum_id: right_enum,
                variant: right_variant,
                payload: right_payload,
                ..
            },
        ) if left_enum == right_enum && left_variant == right_variant => {
            if left_payload.len() != right_payload.len() {
                return Err(Flow::Trap("enum payload shape mismatch".into()));
            }
            for (left, right) in left_payload.iter().zip(right_payload) {
                if !value_equal(left, right)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
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
