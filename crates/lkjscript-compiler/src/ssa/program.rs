use crate::ssa::*;

pub(in crate::ssa) fn construct_program(program: &hir::Program) -> Result<Program> {
    let product_ids: HashMap<String, ProductId> = program
        .products
        .iter()
        .map(|product| (product.name.clone(), ProductId::new(product.id.raw())))
        .collect();
    let function_ids: HashMap<BindingId, FunctionId> = program
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| {
            let raw = u32::try_from(index).unwrap_or(u32::MAX);
            (function.binding, FunctionId::new(raw))
        })
        .collect();
    let function_effects: HashMap<FunctionId, EffectSet> = program
        .functions
        .iter()
        .filter_map(|function| {
            function_ids
                .get(&function.binding)
                .copied()
                .map(|id| (id, effects(function.summary)))
        })
        .collect();

    let mut functions = Vec::with_capacity(program.functions.len().saturating_add(1));
    for function in &program.functions {
        let id = function_ids
            .get(&function.binding)
            .copied()
            .ok_or_else(|| {
                Error::msg(format!(
                    "HIR function binding {} has no SSA FunctionId",
                    function.binding.raw()
                ))
            })?;
        let binding = program.binding(function.binding).ok_or_else(|| {
            Error::msg(format!(
                "HIR function binding {} is missing",
                function.binding.raw()
            ))
        })?;
        let mut signature = signature_from_type(&binding.ty, &product_ids)?;
        signature.bounds = function
            .bounds
            .iter()
            .map(|bound| TraitBound {
                parameter: bound.parameter.clone(),
                trait_id: TraitId::new(bound.trait_id.raw()),
            })
            .collect();
        let mut builder = FunctionBuilder::new(
            &product_ids,
            &function_ids,
            &function_effects,
            id,
            binding.name.clone(),
            signature,
            effects(function.summary),
            origin(function.origin.raw(), 0),
        );
        let entry = builder.new_block(origin(function.origin.raw(), 0), false)?;
        builder.entry = entry;
        builder.current = Some(entry);
        if function.params.len() != builder.signature.parameters.len()
            || function.params.len() != function.param_places.len()
        {
            return Err(Error::msg(format!(
                "HIR function {} parameter/signature mismatch",
                binding.name
            )));
        }
        for (index, ((binding_id, place), ty)) in function
            .params
            .iter()
            .copied()
            .zip(function.param_places.iter().copied())
            .zip(builder.signature.parameters.clone())
            .enumerate()
        {
            builder.register_place(place, binding_id, ty.clone())?;
            let owner_place = is_owned_buf(&ty).then_some(SsaPlaceId::new(place.raw()));
            let parameter = builder.add_block_parameter(
                entry,
                ty,
                owner_place,
                origin(function.origin.raw(), 0),
            )?;
            builder.env.insert(binding_id, parameter);
            let slot =
                u16::try_from(index).map_err(|_| Error::msg("SSA parameter slot exceeds u16"))?;
            builder.slots.insert(binding_id, slot);
        }
        let body = builder.lower_expr(&function.body)?;
        if let Some(result) = body {
            builder.terminate(Terminator::Return(result))?;
        }
        functions.push(builder.finish()?);
    }

    let main_id = FunctionId::new(
        u32::try_from(functions.len()).map_err(|_| Error::msg("too many SSA functions"))?,
    );
    let main_signature = Signature::monomorphic(
        Vec::new(),
        lower_type(&program.main.return_type, &product_ids)?,
    );
    let mut builder = FunctionBuilder::new(
        &product_ids,
        &function_ids,
        &function_effects,
        main_id,
        "main".into(),
        main_signature,
        effects(program.main.body.effects),
        origin(program.main.origin.raw(), 0),
    );
    let entry = builder.new_block(origin(program.main.origin.raw(), 0), false)?;
    builder.entry = entry;
    builder.current = Some(entry);
    if let Some(result) = builder.lower_expr(&program.main.body)? {
        builder.terminate(Terminator::Return(result))?;
    }
    functions.push(builder.finish()?);

    Ok(Program {
        sources: program
            .sources
            .iter()
            .map(|source| {
                let path = source.path.to_str().ok_or_else(|| {
                    Error::msg(format!(
                        "validated source path is not UTF-8: {:?}",
                        source.path
                    ))
                })?;
                Ok(SourceMetadata {
                    id: source.id.raw(),
                    path: path.to_owned(),
                })
            })
            .collect::<Result<Vec<_>>>()?,
        enums: lower_enums(&program.enums, &product_ids)?,
        products: program
            .products
            .iter()
            .map(|product| {
                Ok(ProductMetadata {
                    id: ProductId::new(product.id.raw()),
                    name: product.name.clone(),
                    fields: product
                        .fields
                        .iter()
                        .map(|field| {
                            Ok(ProductField {
                                name: field.name.clone(),
                                ty: lower_type(&field.ty, &product_ids)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                })
            })
            .collect::<Result<Vec<_>>>()?,
        traits: program
            .traits
            .iter()
            .map(|definition| TraitMetadata {
                id: TraitId::new(definition.id.raw()),
                name: definition.name.clone(),
                role: match definition.core {
                    Some(hir::CoreTrait::Copy) => TraitRole::Copy,
                    Some(hir::CoreTrait::Clone) => TraitRole::Clone,
                    Some(hir::CoreTrait::Drop) => TraitRole::Drop,
                    Some(hir::CoreTrait::Send) => TraitRole::Send,
                    Some(hir::CoreTrait::Sync) => TraitRole::Sync,
                    None => TraitRole::User,
                },
                source: match definition.origin {
                    hir::Origin::Source(source) => Some(source.raw()),
                    hir::Origin::Builtin => None,
                },
            })
            .collect(),
        implementations: program
            .implementations
            .iter()
            .map(|implementation| ImplMetadata {
                id: ImplId::new(implementation.id.raw()),
                trait_id: TraitId::new(implementation.trait_id.raw()),
                product: ProductId::new(implementation.product.raw()),
                source: implementation.origin.raw(),
            })
            .collect(),
        functions,
        main: main_id,
    })
}
