use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_sequences(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<&EvalValue>,
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::Cons => binary(&arguments, |head, tail| {
                let EvalValue::List(tail) = tail else {
                    return Err(Flow::Trap("cons tail is not a list".into()));
                };
                if contains_structural(head) || tail.iter().any(contains_structural) {
                    return Err(Flow::Trap(
                        "legacy list cannot contain structural values".into(),
                    ));
                }
                let mut list = Vec::new();
                list.try_reserve_exact(tail.len().saturating_add(1))
                    .map_err(|_| Flow::Resource("legacy list".into()))?;
                list.push(clone_plain_eval_value(head)?);
                for value in tail {
                    list.push(clone_plain_eval_value(value)?);
                }
                self.allocate()?;
                Ok(EvalValue::List(list))
            }),
            Op::Car => unary(&arguments, |list| match list {
                EvalValue::List(items) => items
                    .first()
                    .ok_or_else(|| Flow::Trap("car expects pair".into()))
                    .and_then(clone_plain_eval_value),
                _ => Err(Flow::Trap("car expects pair".into())),
            }),
            Op::Cdr => unary(&arguments, |list| match list {
                EvalValue::List(items) if !items.is_empty() => {
                    self.allocate()?;
                    let mut tail = Vec::new();
                    tail.try_reserve_exact(items.len() - 1)
                        .map_err(|_| Flow::Resource("legacy list tail".into()))?;
                    for value in &items[1..] {
                        tail.push(clone_plain_eval_value(value)?);
                    }
                    Ok(EvalValue::List(tail))
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
                self.allocate_string(String::new())
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
                let argument = self.config.args.get(index).cloned();
                let argument = argument
                    .map(|text| self.allocate_string(text))
                    .transpose()?;
                self.allocate_option(result_type, argument)
            }),
            _ => unreachable!("runtime operation dispatched to the wrong family"),
        }
    }
}

fn contains_structural(value: &EvalValue) -> bool {
    match value {
        EvalValue::StructuralOwner(_)
        | EvalValue::StructuralView(_)
        | EvalValue::StructuralUtf8View(_)
        | EvalValue::StructuralDestination(_) => true,
        EvalValue::List(values) | EvalValue::Product(_, values) => {
            values.iter().any(contains_structural)
        }
        EvalValue::Enum { payload, .. } => payload.iter().any(contains_structural),
        _ => false,
    }
}
