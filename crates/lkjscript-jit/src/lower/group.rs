use super::*;

pub(crate) fn lower_baseline_group(
    verified: &VerifiedProgram,
    root: FunctionId,
    limits: BackendLimits,
) -> Result<LoweredGroup, LoweringError> {
    lower_group(verified.program(), root, limits)
}

pub(crate) fn lower_optimizing_group(
    verified: &VerifiedOptimizedProgram,
    root: FunctionId,
    limits: BackendLimits,
) -> Result<LoweredGroup, LoweringError> {
    lower_group(verified.program(), root, limits)
}

pub(super) fn lower_group(
    program: &lkjscript_ir::Program,
    root: FunctionId,
    limits: BackendLimits,
) -> Result<LoweredGroup, LoweringError> {
    let functions = reachable_group(program, root)?;
    let layouts = LayoutInterner::build(program, &functions)?;
    for function in &functions {
        let item = source_function(program, *function)?;
        preflight_function(program, item, &layouts)?;
    }

    let mut plan = MachinePlanBuilder::new();
    let mut native_functions = Vec::with_capacity(functions.len());
    for function in &functions {
        let item = source_function(program, *function)?;
        let signature = lower_signature(*function, &item.signature, &layouts)?;
        let native = plan
            .declare_function(SourceFunctionId::new(function.raw()), signature)
            .map_err(LoweringError::backend)?;
        native_functions.push((*function, native));
    }

    let mut explicit_traps = Vec::new();
    for function in &functions {
        let item = source_function(program, *function)?;
        let native = native_function(&native_functions, *function)?;
        let mut builder = plan
            .function_builder(native)
            .map_err(LoweringError::backend)?;
        lower_function(
            program,
            item,
            &native_functions,
            &layouts,
            &mut builder,
            &mut explicit_traps,
        )?;
        plan.define_function(builder.finish())
            .map_err(LoweringError::backend)?;
    }

    let verified_plan = plan.verify(limits).map_err(LoweringError::backend)?;
    let image =
        lkjscript_native::encode(verified_plan, lkjscript_native::EncodingConfig::default())
            .map_err(LoweringError::backend)?;
    Ok(LoweredGroup {
        image,
        functions,
        native_functions,
        explicit_traps,
    })
}
