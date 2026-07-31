use super::*;

impl Evaluator<'_> {
    pub(super) fn product_instruction(
        &mut self,
        instruction: &Instruction,
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        match &instruction.kind {
            InstructionKind::ProductValue { product, fields } => match aggregate_mode(
                self.program.program(),
                self.config.structural_limits,
                &instruction.ty,
            )
            .map_err(Flow::Trap)?
            {
                AggregateMode::Structural => self.structural_product(*product, fields, values),
                AggregateMode::Legacy | AggregateMode::ResourceAdapter => {
                    self.charge_aggregate()?;
                    self.allocate()?;
                    Ok(EvalValue::Product(*product, values_for(values, fields)?))
                }
            },
            InstructionKind::ProductField {
                product,
                field,
                value: product_value,
            } => {
                let input = value(values, *product_value)?;
                if matches!(
                    input,
                    EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_)
                ) {
                    self.structural_product_field(*product, *field, input)
                } else {
                    match input {
                        EvalValue::Product(actual, fields) if actual == product => fields
                            .get(usize::from(*field))
                            .ok_or_else(|| Flow::Trap("product field out of bounds".into()))
                            .and_then(clone_plain_eval_value),
                        _ => Err(Flow::Trap("product field identity mismatch".into())),
                    }
                }
            }
            InstructionKind::WithProductField {
                product,
                field,
                value: product_value,
                replacement,
            } => self.with_product_field(
                *product,
                *field,
                value(values, *product_value)?,
                value(values, *replacement)?,
            ),
            _ => Err(Flow::Trap("product instruction dispatch mismatch".into())),
        }
    }

    fn with_product_field(
        &mut self,
        product: ProductId,
        field: u8,
        input: &EvalValue,
        replacement: &EvalValue,
    ) -> Result<EvalValue, Flow> {
        if matches!(
            input,
            EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_)
        ) {
            return self.structural_with_product_field(product, field, input, replacement);
        }
        let EvalValue::Product(actual, fields) = input else {
            return Err(Flow::Trap("product replacement identity mismatch".into()));
        };
        if *actual != product {
            return Err(Flow::Trap("product replacement identity mismatch".into()));
        }
        let mut copied = Vec::new();
        copied
            .try_reserve_exact(fields.len())
            .map_err(|_| Flow::Resource("product replacement".into()))?;
        for value in fields {
            copied.push(clone_plain_eval_value(value)?);
        }
        let Some(slot) = copied.get_mut(usize::from(field)) else {
            return Err(Flow::Trap("product replacement field out of bounds".into()));
        };
        *slot = clone_plain_eval_value(replacement)?;
        self.charge_aggregate()?;
        self.allocate()?;
        Ok(EvalValue::Product(product, copied))
    }
}
