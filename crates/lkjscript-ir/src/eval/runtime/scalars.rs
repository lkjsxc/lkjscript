use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_scalars(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<&EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::Add | Op::Subtract | Op::Multiply | Op::Divide => {
                self.numeric(operation, &arguments)
            }
            Op::EqualValue => binary(&arguments, |left, right| {
                let equal = match self.structural_value_equal(left, right) {
                    Some(result) => result?,
                    None => value_equal(left, right)?,
                };
                Ok(EvalValue::Bool(equal))
            }),
            Op::SameObject => binary(&arguments, |left, right| {
                let same = match (left, right) {
                    (EvalValue::Resource(left), EvalValue::Resource(right)) => {
                        left.same_identity(right)
                    }
                    _ => return Err(Flow::Trap("same-object category mismatch".into())),
                };
                Ok(EvalValue::Bool(same))
            }),
            Op::ListEqual => binary(&arguments, |left, right| {
                let (EvalValue::SegmentedList(left), EvalValue::SegmentedList(right)) =
                    (left, right)
                else {
                    return Err(Flow::Trap("list-equal category mismatch".into()));
                };
                self.segmented_list_equal(*left, *right)
                    .map(EvalValue::Bool)
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
    fn segmented_list_equal(
        &self,
        mut left: lkjscript_core::SegmentedListKey,
        mut right: lkjscript_core::SegmentedListKey,
    ) -> std::result::Result<bool, Flow> {
        let mut steps = 0_usize;
        loop {
            let left_view = self
                .lists
                .view(left)
                .map_err(|error| Flow::Trap(format!("left segmented list: {error:?}")))?;
            let right_view = self
                .lists
                .view(right)
                .map_err(|error| Flow::Trap(format!("right segmented list: {error:?}")))?;
            let (Some((left_value, left_tail)), Some((right_value, right_tail))) =
                (left_view, right_view)
            else {
                return Ok(left_view.is_none() && right_view.is_none());
            };
            if steps >= self.config.max_list_equal_steps {
                return Err(Flow::Trap("list-equal step limit exceeded".into()));
            }
            if !value_equal(left_value, right_value)? {
                return Ok(false);
            }
            left = left_tail;
            right = right_tail;
            steps = steps
                .checked_add(1)
                .ok_or_else(|| Flow::Trap("list-equal step overflow".into()))?;
        }
    }

    pub(crate) fn numeric(
        &self,
        operation: RuntimeOp,
        arguments: &[&EvalValue],
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
        arguments: &[&EvalValue],
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
