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
                AggregateMode::Region => self.region_product(*product, fields, values),
                AggregateMode::Legacy | AggregateMode::ResourceAdapter => {
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
                        EvalValue::RegionProduct(key) => {
                            self.region_product_field(*product, *field, *key)
                        }
                        EvalValue::Product(actual, fields) if actual == product => fields
                            .get(usize::try_from(*field).map_err(|_| {
                                Flow::Trap("product field exceeds host width".into())
                            })?)
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
        field: u64,
        input: &EvalValue,
        replacement: &EvalValue,
    ) -> Result<EvalValue, Flow> {
        if matches!(
            input,
            EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_)
        ) {
            return self.structural_with_product_field(product, field, input, replacement);
        }
        if let EvalValue::RegionProduct(key) = input {
            let identity = self.region_product_identity(product)?;
            let replacement = clone_plain_eval_value(replacement)?;
            let fields = self
                .region_products
                .fields(*key, identity)
                .map_err(region_product_error)?;
            let mut copied = Vec::new();
            copied
                .try_reserve_exact(fields.len())
                .map_err(|_| Flow::Resource("region product replacement".into()))?;
            for value in fields {
                copied.push(clone_plain_eval_value(value)?);
            }
            let field = usize::try_from(field)
                .map_err(|_| Flow::Trap("product field exceeds host width".into()))?;
            let Some(slot) = copied.get_mut(field) else {
                return Err(Flow::Trap("product replacement field out of bounds".into()));
            };
            *slot = replacement;
            self.allocate_dynamic(
                copied
                    .len()
                    .saturating_mul(std::mem::size_of::<EvalValue>()),
            )?;
            let key = self
                .region_products
                .publish(identity, copied)
                .map_err(region_product_error)?;
            return Ok(EvalValue::RegionProduct(key));
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
        let field = usize::try_from(field)
            .map_err(|_| Flow::Trap("product field exceeds host width".into()))?;
        let Some(slot) = copied.get_mut(field) else {
            return Err(Flow::Trap("product replacement field out of bounds".into()));
        };
        *slot = clone_plain_eval_value(replacement)?;
        self.allocate()?;
        Ok(EvalValue::Product(product, copied))
    }

    fn region_product(
        &mut self,
        product: ProductId,
        fields: &[ValueId],
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        let identity = self.region_product_identity(product)?;
        let fields = values_for(values, fields)?;
        self.allocate_dynamic(
            fields
                .len()
                .saturating_mul(std::mem::size_of::<EvalValue>()),
        )?;
        self.region_products
            .publish(identity, fields)
            .map(EvalValue::RegionProduct)
            .map_err(region_product_error)
    }

    fn region_product_field(
        &self,
        product: ProductId,
        field: u64,
        key: lkjscript_core::RegionProductKey,
    ) -> Result<EvalValue, Flow> {
        let identity = self.region_product_identity(product)?;
        self.region_products
            .field(
                key,
                identity,
                usize::try_from(field)
                    .map_err(|_| Flow::Trap("product field exceeds host width".into()))?,
            )
            .map_err(region_product_error)
            .and_then(clone_plain_eval_value)
    }

    fn region_product_identity(
        &self,
        product: ProductId,
    ) -> Result<lkjscript_core::RuntimeLayoutId, Flow> {
        self.program
            .program()
            .region_products
            .iter()
            .find(|metadata| metadata.product == product)
            .map(|metadata| lkjscript_core::RuntimeLayoutId::new(metadata.identity.bytes()))
            .ok_or_else(|| Flow::Trap("region product metadata is missing".into()))
    }
}

fn region_product_error(error: lkjscript_core::RegionProductError) -> Flow {
    match error {
        lkjscript_core::RegionProductError::Records
        | lkjscript_core::RegionProductError::Fields
        | lkjscript_core::RegionProductError::HostAllocation => {
            Flow::Resource("region product".into())
        }
        _ => Flow::Trap(format!("region product: {error:?}")),
    }
}
