fn expected_product_instruction_effects(
    program: &Program,
    instruction: &Instruction,
    types: &[SsaType],
    kind: &InstructionKind,
) -> crate::Result<EffectSet> {
    match kind {
        InstructionKind::ProductValue { product, fields } => {
            if program.memory.type_for(&instruction.ty).is_some() {
                return fail("SSA ProductValue cannot construct a type with structural metadata");
            }
            let metadata = product_by_id(program, *product)?;
            if !is_region_product(program, *product) {
                return fail("SSA ProductValue requires invocation-region product metadata");
            }
            if instruction.ty != SsaType::Product(*product) || fields.len() != metadata.fields.len()
            {
                return fail(format!(
                    "SSA value {} malformed product construction",
                    instruction.id.raw()
                ));
            }
            for (value, field) in fields.iter().zip(&metadata.fields) {
                if value_type(types, *value)? != &field.ty {
                    return fail(format!(
                        "SSA value {} product field type mismatch",
                        instruction.id.raw()
                    ));
                }
            }
            Ok(EffectSet::ALLOCATES)
        }
        InstructionKind::ProductField {
            product,
            field,
            value,
        } => {
            let metadata = product_by_id(program, *product)?;
            if !has_product_storage(program, *product) {
                return fail("SSA product field has no deterministic storage metadata");
            }
            let Some(field_metadata) = metadata.fields.get(usize::from(*field)) else {
                return fail("SSA product field index is out of range");
            };
            if value_type(types, *value)? != &SsaType::Product(*product)
                || instruction.ty != field_metadata.ty
            {
                return fail("SSA product field type or identity mismatch");
            }
            if program
                .memory
                .type_for(&SsaType::Product(*product))
                .is_some()
            {
                Ok(EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES))
            } else {
                Ok(EffectSet::READS_MEMORY)
            }
        }
        InstructionKind::WithProductField {
            product,
            field,
            value,
            replacement,
        } => {
            let metadata = product_by_id(program, *product)?;
            if !has_product_storage(program, *product) {
                return fail("SSA product update has no deterministic storage metadata");
            }
            let Some(field_metadata) = metadata.fields.get(usize::from(*field)) else {
                return fail("SSA replacement field index is out of range");
            };
            if value_type(types, *value)? != &SsaType::Product(*product)
                || value_type(types, *replacement)? != &field_metadata.ty
                || instruction.ty != SsaType::Product(*product)
            {
                return fail("SSA product replacement type or identity mismatch");
            }
            Ok(EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES))
        }
        _ => unreachable!("product instruction family checked"),
    }
}

fn is_region_product(program: &Program, product: ProductId) -> bool {
    program
        .region_products
        .iter()
        .any(|metadata| metadata.product == product)
}

fn has_product_storage(program: &Program, product: ProductId) -> bool {
    is_region_product(program, product)
        || program
            .memory
            .type_for(&SsaType::Product(product))
            .is_some()
}
