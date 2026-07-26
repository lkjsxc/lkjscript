use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_sequences(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::Cons => binary(&arguments, |head, tail| {
                let EvalValue::List(tail) = tail else {
                    return Err(Flow::Trap("cons tail is not a list".into()));
                };
                let mut list = Vec::with_capacity(tail.len().saturating_add(1));
                list.push(head.clone());
                list.extend(tail.iter().cloned());
                self.allocate()?;
                Ok(EvalValue::List(list))
            }),
            Op::Car => unary(&arguments, |list| match list {
                EvalValue::List(items) => items
                    .first()
                    .cloned()
                    .ok_or_else(|| Flow::Trap("car expects pair".into())),
                _ => Err(Flow::Trap("car expects pair".into())),
            }),
            Op::Cdr => unary(&arguments, |list| match list {
                EvalValue::List(items) if !items.is_empty() => {
                    self.allocate()?;
                    Ok(EvalValue::List(items[1..].to_vec()))
                }
                EvalValue::List(_) => Err(Flow::Trap("cdr expects pair".into())),
                _ => Err(Flow::Trap("cdr expects pair".into())),
            }),
            Op::IsEmptyList => unary(&arguments, |list| match list {
                EvalValue::List(items) => Ok(EvalValue::Bool(items.is_empty())),
                _ => Err(Flow::Trap("empty-list? operand is not a list".into())),
            }),
            Op::EmptyStr => {
                exact_arity(&arguments, 0)?;
                self.allocate()?;
                Ok(EvalValue::Str(String::new()))
            }
            Op::ArgCount => {
                exact_arity(&arguments, 0)?;
                let count = i64::try_from(self.config.args.len())
                    .map_err(|_| Flow::Trap("argument count out of range".into()))?;
                Ok(EvalValue::I64(count))
            }
            Op::Arg => unary(&arguments, |index| {
                let index = usize::try_from(as_i64(index)?)
                    .map_err(|_| Flow::Trap("argument index out of range".into()))?;
                let argument = self.config.args.get(index).cloned().map(EvalValue::Str);
                self.allocate_option(argument)
            }),
            _ => unreachable!("runtime operation dispatched to the wrong family"),
        }
    }
}
