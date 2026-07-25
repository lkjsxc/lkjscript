use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_containers(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::Ok => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Ok(Box::new(value.clone())))
            }),
            Op::Err => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Err(Box::new(value.clone())))
            }),
            Op::IsOk => unary(&arguments, |value| match value {
                EvalValue::Ok(_) => Ok(EvalValue::Bool(true)),
                EvalValue::Err(_) => Ok(EvalValue::Bool(false)),
                _ => Err(Flow::Trap("is-ok operand is not Result".into())),
            }),
            Op::UnwrapOk => unary(&arguments, |value| match value {
                EvalValue::Ok(value) => Ok(value.as_ref().clone()),
                EvalValue::Err(error) => {
                    let message = match error.as_ref() {
                        EvalValue::Str(message) => format!("unwrap-ok: {message}"),
                        _ => "unwrap-ok on Err".into(),
                    };
                    Err(Flow::Trap(message))
                }
                _ => Err(Flow::Trap("unwrap-ok operand is not Result".into())),
            }),
            Op::UnwrapErr => unary(&arguments, |value| match value {
                EvalValue::Err(value) => Ok(value.as_ref().clone()),
                EvalValue::Ok(_) => Err(Flow::Trap("unwrap-err on Ok".into())),
                _ => Err(Flow::Trap("unwrap-err operand is not Result".into())),
            }),
            Op::Some => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Some(Box::new(value.clone())))
            }),
            Op::IsSome => unary(&arguments, |value| match value {
                EvalValue::Some(_) => Ok(EvalValue::Bool(true)),
                EvalValue::None => Ok(EvalValue::Bool(false)),
                _ => Err(Flow::Trap("is-some operand is not Option".into())),
            }),
            Op::UnwrapSome => unary(&arguments, |value| match value {
                EvalValue::Some(value) => Ok(value.as_ref().clone()),
                EvalValue::None => Err(Flow::Trap("unwrap-some on none".into())),
                _ => Err(Flow::Trap("unwrap-some operand is not Option".into())),
            }),
            _ => unreachable!("runtime operation dispatched to the wrong family"),
        }
    }
}
