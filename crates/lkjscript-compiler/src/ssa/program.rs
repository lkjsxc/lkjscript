use crate::ssa::*;

pub(in crate::ssa) fn construct_program(
    program: &hir::Program,
    memory_plan: &HirMemoryPlan,
) -> Result<Program> {
    let product_ids: HashMap<crate::hir::ProductId, ProductId> = program
        .products
        .iter()
        .map(|product| (product.id, ProductId::new(product.id.raw())))
        .collect();
    let function_ids: HashMap<BindingId, FunctionId> = program
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| {
            let raw = u64::try_from(index)
                .map_err(|_| Error::msg("SSA function identity exceeds u64"))?;
            Ok((function.binding, FunctionId::new(raw)))
        })
        .collect::<Result<_>>()?;
    let mut structural = lower_structural_memory(program, memory_plan, &product_ids)?;
    let region_products = lower_region_products(program, memory_plan, &product_ids)?;
    if !region_products.is_empty() && structural.types.is_empty() {
        structural.plan = lkjscript_ir::MemoryPlanId::new(memory_plan.id.as_bytes());
    }
    let function_parameter_modes = parameter_modes(&function_ids, memory_plan)?;
    let function_witness_parameters = witness_parameters(memory_plan)?;
    let function_effects = function_effects(program, &function_ids);

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
        signature.memory_witness_parameters = function_witness_parameters
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::msg("HIR function lost verified memory witness parameters"))?;
        let mut builder = FunctionBuilder::new(
            &product_ids,
            &function_ids,
            &function_effects,
            &function_parameter_modes,
            &function_witness_parameters,
            &structural,
            id,
            binding.name.clone(),
            signature,
            effects(function.summary),
            origin(function.origin, 0),
            CleanupPlan::new(
                memory_plan,
                MemoryFunctionId::new(id.raw()),
                &product_ids,
                &structural,
            )?,
        );
        let entry = builder.new_block(origin(function.origin, 0), false)?;
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
            let owner_place = builder
                .owned_place_for_binding(binding_id)?
                .filter(|_| is_owned_value(&structural, &ty));
            let parameter =
                builder.add_block_parameter(entry, ty, owner_place, origin(function.origin, 0))?;
            builder.env.insert(binding_id, parameter);
            if owner_place.is_some() {
                builder.mark_entry_owner(binding_id);
            }
            let slot =
                u64::try_from(index).map_err(|_| Error::msg("SSA parameter slot exceeds u64"))?;
            builder.slots.insert(binding_id, slot);
        }
        builder.install_dynamic_owner_parameters(&function.params, function.origin)?;
        let body = builder.lower_expr(&function.body)?;
        if let Some(result) = body {
            builder.drop_abandoned_structural_owners(result, function.origin)?;
            builder.cleanup_all_places(function.origin)?;
            builder.terminate(Terminator::Return(result))?;
        }
        functions.push(builder.finish()?);
    }

    let main_id = FunctionId::new(
        u64::try_from(functions.len()).map_err(|_| Error::msg("too many SSA functions"))?,
    );
    functions.push(super::entry_function::construct(
        program,
        &product_ids,
        &function_ids,
        &function_effects,
        &function_parameter_modes,
        &function_witness_parameters,
        main_id,
        memory_plan,
        &structural,
    )?);

    let enums = lower_enums(&program.enums, &product_ids)?;
    Ok(Program {
        memory: structural,
        region_products,
        sources: program
            .sources
            .iter()
            .map(lower_source_metadata)
            .collect::<Result<Vec<_>>>()?,
        enums,
        products: lower_product_metadata(program, &product_ids)?,
        traits: lower_trait_metadata(program),
        implementations: lower_implementation_metadata(program),
        functions,
        main: main_id,
    })
}

include!("program/maps.rs");
include!("program/source.rs");
include!("program/metadata.rs");
