use super::*;

impl Evaluator<'_> {
    pub(crate) fn instruction(
        &mut self,
        instruction: &Instruction,
        values: &[Option<EvalValue>],
        depth: usize,
    ) -> std::result::Result<EvalValue, Flow> {
        match &instruction.kind {
            InstructionKind::Constant(constant) => self.constant(constant),
            InstructionKind::Copy(source)
            | InstructionKind::Move { value: source, .. }
            | InstructionKind::Borrow { value: source, .. } => value(values, *source).cloned(),
            InstructionKind::PlaceInit { .. }
            | InstructionKind::PlaceEnd { .. }
            | InstructionKind::EndBorrow { .. }
            | InstructionKind::Drop { .. } => Ok(EvalValue::Unit),
            InstructionKind::FunctionRef(function) => Ok(EvalValue::Function(*function)),
            InstructionKind::Runtime {
                operation,
                arguments,
                ..
            } => {
                let arguments = values_for(values, arguments)?;
                self.runtime(*operation, arguments)
            }
            kind @ (InstructionKind::F64FromI64Exact { value: input }
            | InstructionKind::F64FromI64Rounded { value: input }
            | InstructionKind::I64FromF64Exact { value: input }
            | InstructionKind::I64FromF64Trunc { value: input }) => {
                self.numeric_conversion(kind, value(values, *input)?)
            }
            InstructionKind::Call {
                target, arguments, ..
            } => {
                let target = match target {
                    CallTarget::Direct(function) => *function,
                    CallTarget::Indirect(target) => match value(values, *target)? {
                        EvalValue::Function(function) => *function,
                        _ => {
                            return Err(Flow::Trap(
                                "evaluator call target is not a function".into(),
                            ))
                        }
                    },
                };
                let arguments = values_for(values, arguments)?;
                self.call(target, arguments, depth.saturating_add(1))
            }
            InstructionKind::ProductValue { product, fields } => {
                self.allocate()?;
                Ok(EvalValue::Product(*product, values_for(values, fields)?))
            }
            InstructionKind::ProductField {
                product,
                field,
                value: product_value,
            } => match value(values, *product_value)? {
                EvalValue::Product(actual, fields) if actual == product => fields
                    .get(usize::from(*field))
                    .cloned()
                    .ok_or_else(|| Flow::Trap("product field out of bounds".into())),
                _ => Err(Flow::Trap("product field identity mismatch".into())),
            },
            InstructionKind::WithProductField {
                product,
                field,
                value: product_value,
                replacement,
            } => match value(values, *product_value)? {
                EvalValue::Product(actual, fields) if actual == product => {
                    let mut fields = fields.clone();
                    let Some(slot) = fields.get_mut(usize::from(*field)) else {
                        return Err(Flow::Trap("product replacement field out of bounds".into()));
                    };
                    *slot = value(values, *replacement)?.clone();
                    self.allocate()?;
                    Ok(EvalValue::Product(*product, fields))
                }
                _ => Err(Flow::Trap("product replacement identity mismatch".into())),
            },
            InstructionKind::EnumValue {
                enum_id,
                variant,
                layout,
                fields,
            } => {
                let definition = self
                    .program
                    .program()
                    .enums
                    .iter()
                    .find(|definition| definition.id == *enum_id)
                    .ok_or_else(|| Flow::Trap("enum metadata missing".into()))?;
                let selected = definition
                    .variants
                    .iter()
                    .find(|candidate| candidate.id == *variant)
                    .ok_or_else(|| Flow::Trap("enum variant metadata missing".into()))?;
                self.charge_aggregate()?;
                self.allocate()?;
                Ok(EvalValue::Enum {
                    enum_id: *enum_id,
                    variant: *variant,
                    layout: *layout,
                    physical_tag: selected.physical_tag,
                    payload: values_for(values, fields)?,
                })
            }
            InstructionKind::EnumIsVariant {
                enum_id,
                variant,
                layout,
                value: input,
            } => match value(values, *input)? {
                EvalValue::Enum {
                    enum_id: actual,
                    variant: active,
                    layout: actual_layout,
                    ..
                } if actual == enum_id && actual_layout == layout => {
                    Ok(EvalValue::Bool(active == variant))
                }
                _ => Err(Flow::Trap(
                    "enum variant test identity/layout mismatch".into(),
                )),
            },
            InstructionKind::EnumField {
                enum_id,
                variant,
                field,
                layout,
                value: input,
            } => {
                let definition = self
                    .program
                    .program()
                    .enums
                    .iter()
                    .find(|definition| definition.id == *enum_id)
                    .ok_or_else(|| Flow::Trap("enum metadata missing".into()))?;
                let selected = definition
                    .variants
                    .iter()
                    .find(|candidate| candidate.id == *variant)
                    .ok_or_else(|| Flow::Trap("enum variant metadata missing".into()))?;
                let index = selected
                    .fields
                    .iter()
                    .position(|candidate| candidate.id == *field)
                    .ok_or_else(|| Flow::Trap("enum field metadata missing".into()))?;
                match value(values, *input)? {
                    EvalValue::Enum {
                        enum_id: actual,
                        variant: active,
                        layout: actual_layout,
                        payload,
                        ..
                    } if actual == enum_id && active == variant && actual_layout == layout => {
                        payload
                            .get(index)
                            .cloned()
                            .ok_or_else(|| Flow::Trap("enum active payload is malformed".into()))
                    }
                    EvalValue::Enum { .. } => Err(Flow::Trap("inactive enum projection".into())),
                    _ => Err(Flow::Trap("enum projection expects enum".into())),
                }
            }
        }
    }

    pub(crate) fn constant(&mut self, constant: &Constant) -> std::result::Result<EvalValue, Flow> {
        match constant {
            Constant::Unit => Ok(EvalValue::Unit),
            Constant::Bool(value) => Ok(EvalValue::Bool(*value)),
            Constant::I64(value) => Ok(EvalValue::I64(*value)),
            Constant::F64(value) => Ok(EvalValue::F64(*value)),
            Constant::Str(value) => {
                self.allocate()?;
                Ok(EvalValue::Str(value.clone()))
            }
            Constant::Symbol(value) => {
                self.allocate()?;
                Ok(EvalValue::Symbol(value.clone()))
            }
            Constant::EmptyList => Ok(EvalValue::List(Vec::new())),
        }
    }
}
