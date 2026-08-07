use super::super::*;

pub(crate) const fn evaluator_runtime_value_bytes() -> usize {
    // Deterministic base estimate for one evaluator-owned aggregate record.
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
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Flow::Resource("heap bytes".into()))?;
    pending.push((left, right));
    while let Some((left, right)) = pending.pop() {
        match (left, right) {
            (EvalValue::Unit, EvalValue::Unit) => {}
            (EvalValue::Bool(left), EvalValue::Bool(right)) if left == right => {}
            (EvalValue::I64(left), EvalValue::I64(right)) if left == right => {}
            (EvalValue::F64(left), EvalValue::F64(right)) if left == right => {}
            (EvalValue::StaticSymbol(left), EvalValue::StaticSymbol(right)) if left == right => {}
            (EvalValue::Symbol(left), EvalValue::Symbol(right)) if left == right => {}
            (EvalValue::Bool(_), EvalValue::Bool(_))
            | (EvalValue::I64(_), EvalValue::I64(_))
            | (EvalValue::F64(_), EvalValue::F64(_))
            | (EvalValue::StaticSymbol(_), EvalValue::StaticSymbol(_))
            | (EvalValue::Symbol(_), EvalValue::Symbol(_)) => return Ok(false),
            (EvalValue::Resource(_), _) | (_, EvalValue::Resource(_)) => {
                return Err(Flow::Trap(
                    "typed resources cannot be compared as values".into(),
                ));
            }
            (EvalValue::StaticString(_), _)
            | (_, EvalValue::StaticString(_))
            | (EvalValue::StructuralOwner(_), _)
            | (_, EvalValue::StructuralOwner(_))
            | (EvalValue::StructuralView(_), _)
            | (_, EvalValue::StructuralView(_)) => {
                return Err(Flow::Trap(
                    "legacy list cannot contain structural values".into(),
                ));
            }
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
            ) if left_enum == right_enum => {
                if left_variant != right_variant {
                    return Ok(false);
                }
                if left_payload.len() != right_payload.len() {
                    return Err(Flow::Trap("enum payload shape mismatch".into()));
                }
                pending
                    .try_reserve(left_payload.len())
                    .map_err(|_| Flow::Resource("heap bytes".into()))?;
                pending.extend(left_payload.iter().zip(right_payload).rev());
            }
            _ => return Err(Flow::Trap("equal-value category mismatch".into())),
        }
    }
    Ok(true)
}

pub(crate) fn unary<F>(
    arguments: &[&EvalValue],
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
    arguments: &[&EvalValue],
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
    arguments: &[&EvalValue],
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
    arguments: &[&EvalValue],
    expected: usize,
) -> std::result::Result<(), Flow> {
    if arguments.len() == expected {
        Ok(())
    } else {
        Err(Flow::Trap("evaluator runtime arity mismatch".into()))
    }
}
