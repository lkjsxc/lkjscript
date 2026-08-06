impl TypePlanner<'_> {
    fn charge_fields(&mut self, amount: usize) -> Result<()> {
        checked_observe(&mut self.fields, amount, "aggregate fields")
    }
    fn charge_variants(&mut self, amount: usize) -> Result<()> {
        checked_observe(&mut self.variants, amount, "aggregate variants")
    }
}

impl Producer<'_> {
    fn reject_affine_load(&mut self, expression: &Expr, binding: BindingId) -> Result<()> {
        let type_fact = self.type_planner.intern(&expression.ty)?;
        let fact = self.type_planner.fact(type_fact)?;
        if matches!(expression.ty, Type::Product(_) | Type::Enum { .. })
            && fact.mode == MemoryAggregateMode::Affine {
            return Err(Error::msg(format!(
                "LKJ-MEM-AFFINE-AGGREGATE-COPY type={:?} binding={}",
                memory_type(&expression.ty), binding.raw())));
        }
        Ok(())
    }
}

impl Producer<'_> {
    fn add_use(&mut self, expression: MemoryExpressionId, binding: BindingId,
        kind: MemoryUseKind) -> Result<()> {
        self.charge_uses(1)?;
        self.uses
            .try_reserve(1)
            .map_err(|_| Error::host("HIR memory-plan use allocation failed"))?;
        let id = MemoryUseId::new(u32::try_from(self.uses.len())
            .map_err(|_| Error::msg("HIR memory-plan use identity exceeds u32"))?);
        self.uses.push(MemoryUse { id, function: self.current_function,
            expression, binding: binding.raw(), kind });
        Ok(())
    }
}

impl TypePlanner<'_> {
fn recursive_fields(
    &self,
    key: &DeclarationKey,
) -> Result<Vec<(Type, MemoryTypePathElement)>> {
    match key {
        DeclarationKey::Product(name) => {
            let item = self.product_indices.get(name)
                .and_then(|index| self.program.products.get(*index))
                .filter(|item| item.name == *name)
                .ok_or_else(|| Error::msg("recursive memory plan lost product"))?;
            item.fields.iter().enumerate().map(|(index, field)| Ok((field.ty.clone(),
                MemoryTypePathElement::ProductField { index: index_u32(index)?,
                    name: field.name.clone() }))).collect()
        }
        DeclarationKey::Enum(id) => {
            let item = self.enum_indices.get(id)
                .and_then(|index| self.program.enums.get(*index))
                .filter(|item| item.id.bytes() == *id)
                .ok_or_else(|| Error::msg("recursive memory plan lost enum"))?;
            let mut fields = Vec::new();
            for (variant_index, variant) in item.variants.iter().enumerate() {
                for (field_index, field) in variant.fields.iter().enumerate() {
                    fields.push((field.ty.clone(), MemoryTypePathElement::EnumVariantField {
                        variant_index: index_u32(variant_index)?, variant: variant.id.bytes(),
                        field_index: index_u32(field_index)?, field: field.id.bytes(),
                    }));
                }
            }
            Ok(fields)
        }
    }
}

fn recursive_substitutions(
    &self,
    declaration: &DeclarationKey,
    root: &DeclarationKey,
    arguments: &[Type],
) -> Result<HashMap<String, Type>> {
    if declaration != root { return Ok(HashMap::new()); }
    let DeclarationKey::Enum(id) = declaration else { return Ok(HashMap::new()); };
    let item = self.enum_indices.get(id)
        .and_then(|index| self.program.enums.get(*index))
        .filter(|item| item.id.bytes() == *id)
        .ok_or_else(|| Error::msg("recursive memory plan lost enum substitution"))?;
    if item.type_parameters.len() != arguments.len() {
        return Err(Error::msg("recursive memory enum substitution arity mismatch"));
    }
    Ok(item.type_parameters.iter().cloned().zip(arguments.iter().cloned()).collect())
}

fn recursive_root_type(
    &self,
    key: &DeclarationKey,
    arguments: &[Type],
) -> Result<Type> {
    match key {
        DeclarationKey::Product(name) => Ok(Type::Product(name.clone())),
        DeclarationKey::Enum(id) => {
            let item = self.enum_indices.get(id)
                .and_then(|index| self.program.enums.get(*index))
                .filter(|item| item.id.bytes() == *id)
                .ok_or_else(|| Error::msg("recursive memory plan lost enum identity"))?;
            Ok(Type::Enum { id: item.id, name: item.name.clone(), arguments: arguments.to_vec() })
        }
    }
}
}
