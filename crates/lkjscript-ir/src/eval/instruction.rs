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
            InstructionKind::PlaceInit { .. } | InstructionKind::PlaceEnd { .. } => {
                Ok(EvalValue::Unit)
            }
            InstructionKind::FunctionRef(function) => Ok(EvalValue::Function(*function)),
            InstructionKind::Runtime {
                operation,
                arguments,
                ..
            } => {
                let arguments = values_for(values, arguments)?;
                self.runtime(*operation, arguments)
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
            Constant::None => Ok(EvalValue::None),
        }
    }
}
