use super::*;

pub(super) fn lowering_domain(
    program: &lkjscript_ir::Program,
    functions: &[FunctionId],
) -> Result<LoweringDomain, LoweringError> {
    let mut resource = false;
    let mut unique = false;
    let mut structural = false;
    let mut region_product = false;
    let mut legacy_aggregate = false;
    for id in functions {
        let function = source_function(program, *id)?;
        let signature_and_block_types = function
            .signature
            .parameters
            .iter()
            .chain(std::iter::once(function.signature.result.as_ref()))
            .chain(
                function
                    .blocks
                    .iter()
                    .flat_map(|block| block.parameters.iter().map(|value| &value.ty)),
            );
        structural |= signature_and_block_types
            .clone()
            .any(|ty| program.memory.type_for(ty).is_some());
        let types = signature_and_block_types.chain(
            function
                .blocks
                .iter()
                .flat_map(|block| block.instructions.iter().map(|value| &value.ty)),
        );
        for ty in types {
            resource |= contains_capability_or_resource(ty);
            unique |= contains_unique(ty);
            region_product |= matches!(ty, SsaType::Product(product)
                if program.region_products.iter().any(|metadata| metadata.product == *product));
            legacy_aggregate |= matches!(ty, SsaType::Product(product)
                if !program.region_products.iter().any(|metadata| metadata.product == *product)
                    && program.memory.type_for(ty).is_none())
                || matches!(ty, SsaType::Enum { .. }) && program.memory.type_for(ty).is_none();
        }
        structural |= function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                program.memory.type_for(&instruction.ty).is_some()
                    && !matches!(
                        instruction.kind,
                        InstructionKind::Constant(Constant::Str(_))
                    )
            })
        });
        resource |= function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(
                    instruction.kind,
                    InstructionKind::Runtime {
                        operation: RuntimeOp::StdinHandle,
                        ..
                    }
                )
            })
        });
        structural |= function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(
                    instruction.kind,
                    InstructionKind::StructuralPublish { .. }
                        | InstructionKind::DestinationCreate { .. }
                        | InstructionKind::DestinationFieldInit { .. }
                        | InstructionKind::DestinationFinish { .. }
                        | InstructionKind::DestinationAbort { .. }
                        | InstructionKind::AggregateFieldBorrow { .. }
                        | InstructionKind::AggregateTag { .. }
                        | InstructionKind::AggregateConsumePayload { .. }
                        | InstructionKind::StringUtf8View { .. }
                )
            })
        });
    }
    if region_product && (resource || unique || structural || legacy_aggregate) {
        return Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(functions[0]),
            "native invocation-region products cannot bridge another ownership domain",
        ));
    }
    match (resource, unique, structural) {
        (true, true, _) | (true, _, true) | (_, true, true) => Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(functions[0]),
            "native resource, unique, and structural families cannot bridge one verified group",
        )),
        (true, false, false) => Ok(LoweringDomain::ResourceIsland),
        (false, true, false) => Ok(LoweringDomain::UniqueIsland),
        (false, false, true) => Ok(LoweringDomain::StructuralIsland),
        (false, false, false) => Ok(LoweringDomain::Legacy),
    }
}

fn contains_unique(ty: &SsaType) -> bool {
    match ty {
        SsaType::Bytes | SsaType::ByteVector | SsaType::ByteSlice | SsaType::ByteSliceMut => true,
        SsaType::List(inner) => contains_unique(inner),
        SsaType::Enum { arguments, .. } => arguments.iter().any(contains_unique),
        SsaType::Function(signature) => signature
            .parameters
            .iter()
            .chain(std::iter::once(signature.result.as_ref()))
            .any(contains_unique),
        _ => false,
    }
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
        | SsaType::Bytes
        | SsaType::ByteVector
        | SsaType::ByteSlice
        | SsaType::ByteSliceMut
        | SsaType::Path
        | SsaType::StructuralDestination(_)
        | SsaType::Product(_)
        | SsaType::TypeParameter(_) => false,
    }
}
