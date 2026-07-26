use crate::ssa::*;

pub(super) fn construct(
    program: &hir::Program,
    product_ids: &HashMap<String, ProductId>,
    function_ids: &HashMap<BindingId, FunctionId>,
    function_effects: &HashMap<FunctionId, EffectSet>,
    main_id: FunctionId,
) -> Result<Function> {
    let signature = Signature::monomorphic(
        program
            .main
            .param_types
            .iter()
            .map(|ty| lower_type(ty, product_ids))
            .collect::<Result<Vec<_>>>()?,
        lower_type(&program.main.return_type, product_ids)?,
    );
    let mut builder = FunctionBuilder::new(
        product_ids,
        function_ids,
        function_effects,
        main_id,
        "main".into(),
        signature,
        effects(program.main.body.effects),
        origin(program.main.origin.raw(), 0),
    );
    let entry = builder.new_block(origin(program.main.origin.raw(), 0), false)?;
    builder.entry = entry;
    builder.current = Some(entry);
    if program.main.params.len() != builder.signature.parameters.len()
        || program.main.params.len() != program.main.param_places.len()
    {
        return Err(Error::msg("HIR main parameter/signature mismatch"));
    }
    for (index, ((binding, place), ty)) in program
        .main
        .params
        .iter()
        .copied()
        .zip(program.main.param_places.iter().copied())
        .zip(builder.signature.parameters.clone())
        .enumerate()
    {
        builder.register_place(place, binding, ty.clone())?;
        let parameter =
            builder.add_block_parameter(entry, ty, None, origin(program.main.origin.raw(), 0))?;
        builder.env.insert(binding, parameter);
        let slot =
            u16::try_from(index).map_err(|_| Error::msg("SSA main parameter slot exceeds u16"))?;
        builder.slots.insert(binding, slot);
    }
    if let Some(result) = builder.lower_expr(&program.main.body)? {
        builder.terminate(Terminator::Return(result))?;
    }
    builder.finish()
}
