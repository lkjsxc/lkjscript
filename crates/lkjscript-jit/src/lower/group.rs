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
    let domain = lowering_domain(program, &functions)?;
    let layouts = LayoutInterner::build(program, &functions)?;
    for function in &functions {
        let item = source_function(program, *function)?;
        preflight_function(program, item, &layouts, domain)?;
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

fn lowering_domain(
    program: &lkjscript_ir::Program,
    functions: &[FunctionId],
) -> Result<LoweringDomain, LoweringError> {
    for function in functions {
        let function = source_function(program, *function)?;
        if function
            .signature
            .parameters
            .iter()
            .chain(std::iter::once(function.signature.result.as_ref()))
            .any(contains_capability_or_resource)
            || function.blocks.iter().any(|block| {
                block
                    .parameters
                    .iter()
                    .any(|value| contains_capability_or_resource(&value.ty))
                    || block.instructions.iter().any(|instruction| {
                        contains_capability_or_resource(&instruction.ty)
                            || matches!(
                                instruction.kind,
                                InstructionKind::Runtime {
                                    operation: RuntimeOp::StdinHandle,
                                    ..
                                }
                            )
                    })
            })
        {
            return Ok(LoweringDomain::ResourceIsland);
        }
    }
    Ok(LoweringDomain::Legacy)
}

fn contains_capability_or_resource(ty: &SsaType) -> bool {
    match ty {
        SsaType::Capability(_) | SsaType::Resource(_) => true,
        SsaType::List(inner) => contains_capability_or_resource(inner),
        SsaType::Enum { arguments, .. } => arguments.iter().any(contains_capability_or_resource),
        SsaType::Function(signature) => signature
            .parameters
            .iter()
            .chain(std::iter::once(signature.result.as_ref()))
            .any(contains_capability_or_resource),
        SsaType::Unit
        | SsaType::Bool
        | SsaType::I64
        | SsaType::F64
        | SsaType::Str
        | SsaType::Symbol
        | SsaType::Buf
        | SsaType::ByteVector
        | SsaType::ByteSlice
        | SsaType::ByteSliceMut
        | SsaType::Path
        | SsaType::Product(_)
        | SsaType::TypeParameter(_) => false,
    }
}
