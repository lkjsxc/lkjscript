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
                let EvalValue::SegmentedList(tail) = tail else {
                    return Err(Flow::Trap("list-prepend tail is not a list".into()));
                };
                if contains_structural(head) {
                    return Err(Flow::Trap(
                        "segmented list cannot contain an unplanned structural owner".into(),
                    ));
                }
                let head = clone_plain_eval_value(head)?;
                self.allocate()?;
                self.lists
                    .prepend(head, *tail)
                    .map(EvalValue::SegmentedList)
                    .map_err(segmented_list_flow)
            }),
            Op::Car => unary(&arguments, |list| {
                let EvalValue::SegmentedList(key) = list else {
                    return Err(Flow::Trap("list-first expects a list".into()));
                };
                self.lists
                    .first(*key)
                    .map_err(segmented_list_flow)
                    .and_then(clone_plain_eval_value)
            }),
            Op::Cdr => unary(&arguments, |list| {
                let EvalValue::SegmentedList(key) = list else {
                    return Err(Flow::Trap("list-rest expects a list".into()));
                };
                self.lists
                    .rest(*key)
                    .map(EvalValue::SegmentedList)
                    .map_err(segmented_list_flow)
            }),
            Op::IsEmptyList => unary(&arguments, |list| {
                let EvalValue::SegmentedList(key) = list else {
                    return Err(Flow::Trap("is-empty-list operand is not a list".into()));
                };
                self.lists
                    .is_empty(*key)
                    .map(EvalValue::Bool)
                    .map_err(segmented_list_flow)
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
        EvalValue::SegmentedList(_) | EvalValue::RegionProduct(_) => false,
        EvalValue::Enum { payload, .. } => payload.iter().any(contains_structural),
        _ => false,
    }
}

fn segmented_list_flow(error: lkjscript_core::SegmentedListError) -> Flow {
    match error {
        lkjscript_core::SegmentedListError::EmptyList => {
            Flow::Trap("list operation requires a nonempty list".into())
        }
        lkjscript_core::SegmentedListError::Limit(limit) => {
            Flow::Resource(format!("segmented list {limit:?}"))
        }
        _ => Flow::Trap(format!("invalid segmented-list handle: {error:?}")),
    }
}
