use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_scalars(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::Add | Op::Subtract | Op::Multiply | Op::Divide => {
                self.numeric(operation, &arguments)
            }
            Op::EqualValue => binary(&arguments, |left, right| {
                Ok(EvalValue::Bool(value_equal(left, right)?))
            }),
            Op::SameObject => binary(&arguments, |left, right| {
                let same = match (left, right) {
                    (EvalValue::Buf(left), EvalValue::Buf(right)) => left.id == right.id,
                    _ => return Err(Flow::Trap("same-object category mismatch".into())),
                };
                Ok(EvalValue::Bool(same))
            }),
            Op::ListEqual => binary(&arguments, |left, right| {
                let (EvalValue::List(left), EvalValue::List(right)) = (left, right) else {
                    return Err(Flow::Trap("list-equal category mismatch".into()));
                };
                Ok(EvalValue::Bool(list_values_equal(
                    left,
                    right,
                    self.config.max_list_equal_steps,
                )?))
            }),
            Op::F64BitsEqual => binary(&arguments, |left, right| {
                Ok(EvalValue::Bool(
                    as_f64_exact(left)?.to_bits() == as_f64_exact(right)?.to_bits(),
                ))
            }),
            Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual => {
                self.compare(operation, &arguments)
            }
            Op::Not => unary(&arguments, |value| Ok(EvalValue::Bool(!as_bool(value)?))),
            Op::BitAnd | Op::BitOr | Op::BitXor => binary(&arguments, |left, right| {
                let left = as_i64(left)?;
                let right = as_i64(right)?;
                Ok(EvalValue::I64(match operation {
                    Op::BitAnd => left & right,
                    Op::BitOr => left | right,
                    Op::BitXor => left ^ right,
                    _ => return Err(Flow::Trap("invalid bit operation".into())),
                }))
            }),
            _ => unreachable!("runtime operation dispatched to the wrong family"),
        }
    }
    pub(crate) fn numeric(
        &self,
        operation: RuntimeOp,
        arguments: &[EvalValue],
    ) -> std::result::Result<EvalValue, Flow> {
        exact_arity(arguments, 2)?;
        let left = arguments
            .first()
            .ok_or_else(|| Flow::Trap("numeric operand missing".into()))?;
        let right = arguments
            .get(1)
            .ok_or_else(|| Flow::Trap("numeric operand missing".into()))?;
        if matches!(left, EvalValue::F64(_)) || matches!(right, EvalValue::F64(_)) {
            let left = as_numeric_f64(left)?;
            let right = as_numeric_f64(right)?;
            let result = match operation {
                RuntimeOp::Add => left + right,
                RuntimeOp::Subtract => left - right,
                RuntimeOp::Multiply => left * right,
                RuntimeOp::Divide => left / right,
                _ => return Err(Flow::Trap("invalid numeric operation".into())),
            };
            Ok(EvalValue::F64(result))
        } else {
            let left = as_i64(left)?;
            let right = as_i64(right)?;
            let result = match operation {
                RuntimeOp::Add => left.checked_add(right),
                RuntimeOp::Subtract => left.checked_sub(right),
                RuntimeOp::Multiply => left.checked_mul(right),
                RuntimeOp::Divide => left.checked_div(right),
                _ => return Err(Flow::Trap("invalid numeric operation".into())),
            }
            .ok_or_else(|| Flow::Trap("checked I64 arithmetic failed".into()))?;
            Ok(EvalValue::I64(result))
        }
    }

    pub(crate) fn compare(
        &self,
        operation: RuntimeOp,
        arguments: &[EvalValue],
    ) -> std::result::Result<EvalValue, Flow> {
        exact_arity(arguments, 2)?;
        let left = arguments
            .first()
            .ok_or_else(|| Flow::Trap("comparison operand missing".into()))?;
        let right = arguments
            .get(1)
            .ok_or_else(|| Flow::Trap("comparison operand missing".into()))?;
        let result = if matches!(left, EvalValue::F64(_)) || matches!(right, EvalValue::F64(_)) {
            compare_values(operation, as_numeric_f64(left)?, as_numeric_f64(right)?)?
        } else {
            compare_values(operation, as_i64(left)?, as_i64(right)?)?
        };
        Ok(EvalValue::Bool(result))
    }
}
