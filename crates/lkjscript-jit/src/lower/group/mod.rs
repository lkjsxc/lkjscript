use super::*;

mod domain;
use domain::lowering_domain;

pub(crate) fn lower_baseline_group(
    verified: &VerifiedProgram,
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
    let domain = lowering_domain(program, &functions)?;
    let bytes_modes = BytesModes::analyze(program, &functions)?;
    let layouts = LayoutInterner::build(program, &functions)?;
    for function in &functions {
        let item = source_function(program, *function)?;
        preflight_function(program, item, &layouts, &bytes_modes, domain)?;
    }
    let root_function = source_function(program, root)?;
    if matches!(
        root_function.signature.result.as_ref(),
        SsaType::Resource(_)
    ) {
        return Err(LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(root),
            "native resource values cannot escape the selected root",
        ));
    }

    let mut plan = MachinePlanBuilder::new();
    let mut static_bytes = HashMap::new();
    for function in &functions {
        for value in source_function(program, *function)?
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .filter_map(|instruction| match &instruction.kind {
                InstructionKind::Constant(Constant::StaticBytes(bytes)) => Some(bytes.clone()),
                InstructionKind::Constant(Constant::Str(value))
                    if layouts.structural().selected(&instruction.ty) =>
                {
                    Some(value.as_bytes().to_vec())
                }
                _ => None,
            })
        {
            let identity = plan
                .intern_static_bytes(&value)
                .map_err(LoweringError::backend)?;
            static_bytes.insert(value, identity);
        }
    }
    if functions.iter().any(|function| {
        source_function(program, *function).is_ok_and(|function| {
            function.blocks.iter().any(|block| {
                block.instructions.iter().any(|instruction| {
                    matches!(
                        instruction.kind,
                        InstructionKind::Runtime {
                            operation: RuntimeOp::EmptyStr,
                            ..
                        }
                    )
                })
            })
        })
    }) {
        let identity = plan
            .intern_static_bytes(&[])
            .map_err(LoweringError::backend)?;
        static_bytes.insert(Vec::new(), identity);
    }
    let mut native_functions = Vec::with_capacity(functions.len());
    for function in &functions {
        let item = source_function(program, *function)?;
        let signature = lower_signature(item, &bytes_modes, &layouts)?;
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
            &bytes_modes,
            &static_bytes,
            &mut builder,
            &mut explicit_traps,
        )?;
        plan.define_function(builder.finish())
            .map_err(LoweringError::backend)?;
    }

    let verified_plan = plan.verify(limits).map_err(LoweringError::backend)?;
    let image = lkjscript_native::encode(verified_plan).map_err(LoweringError::backend)?;
    Ok(LoweredGroup {
        image,
        functions,
        native_functions,
        explicit_traps,
    })
}
