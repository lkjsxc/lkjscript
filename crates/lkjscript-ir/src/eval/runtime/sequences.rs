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
                reject_affine_list_element(head)?;
                let tracks_owner = list_owned_marker(head).is_some();
                if tracks_owner {
                    self.list_owned
                        .try_reserve(1)
                        .map_err(|_| Flow::Resource("segmented list owner ledger".into()))?;
                }
                let head = self.copy_eval_value(head)?;
                if tracks_owner {
                    let owner = list_owned_marker(&head).ok_or_else(|| {
                        Flow::Trap("segmented list owner changed runtime category".into())
                    })?;
                    self.list_owned.push(owner);
                }
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
                let owner = {
                    let element = self.lists.first(*key).map_err(segmented_list_flow)?;
                    if let Some(owner) = list_owned_marker(element) {
                        owner
                    } else {
                        return clone_plain_eval_value(element);
                    }
                };
                self.copy_eval_value(&owner)
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

fn list_owned_marker(value: &EvalValue) -> Option<EvalValue> {
    match value {
        EvalValue::StructuralOwner(owner) => Some(EvalValue::StructuralOwner(*owner)),
        EvalValue::StructuralView(view) => Some(EvalValue::StructuralView(*view)),
        EvalValue::Path(word) => Some(EvalValue::Path(*word)),
        _ => None,
    }
}

fn reject_affine_list_element(value: &EvalValue) -> Result<(), Flow> {
    if matches!(
        value,
        EvalValue::StructuralUtf8View(_)
            | EvalValue::StructuralDestination(_)
            | EvalValue::Bytes(_)
            | EvalValue::BytesBorrow(_)
            | EvalValue::ByteVector(_)
            | EvalValue::ByteSlice(_)
            | EvalValue::ByteSliceMut(_)
            | EvalValue::Resource(_)
            | EvalValue::ReturnedOwned(_)
            | EvalValue::ReturnedByteVector(_)
            | EvalValue::ReturnedBytes(_)
    ) {
        Err(Flow::Trap(
            "segmented list cannot contain an affine, borrowed, or boundary value".into(),
        ))
    } else {
        Ok(())
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
