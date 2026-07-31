use super::*;

pub fn encode(
    plan: VerifiedMachinePlan,
    config: EncodingConfig,
) -> Result<InstallableImage, NativeError> {
    let mut bytes = Vec::new();
    let mut entries = Vec::new();
    let mut relocations = Vec::new();
    let mut runtime_call_set = HashSet::new();
    let mut frames = Vec::new();
    let mut safepoints = Vec::new();
    let mut root_requirements = Vec::new();
    let mut heap_runtime_sites = Vec::new();
    let mut structural_runtime_sites = Vec::new();
    let mut source_map = Vec::new();
    let mut trap_map = Vec::new();
    let mut outcome_map = Vec::new();
    let signatures: Vec<_> = plan
        .functions
        .iter()
        .map(|function| (function.id, function.signature.clone()))
        .collect();
    let execution_domain = execution_domain(&plan.functions);
    let collecting_functions = collecting_function_closure(&plan.functions);

    for (function_ordinal, function) in plan.functions.iter().enumerate() {
        let start = bytes.len();
        let frame_bytes = calculate_frame_bytes(function)?;
        let outgoing_arguments = maximum_outgoing_arguments(function)?;
        let certified_call_roots = plan
            .root_requirements
            .get(function_ordinal)
            .ok_or(NativeError::Encode(EncodeError::InvalidCall))?;
        let function_ordinal = to_u32(function_ordinal)?;
        let mut encoder = FunctionEncoder {
            function,
            function_ordinal,
            signatures: &signatures,
            execution_domain,
            collecting_functions: &collecting_functions,
            bytes: &mut bytes,
            relocations: &mut relocations,
            safepoints: &mut safepoints,
            root_requirements: &mut root_requirements,
            heap_runtime_sites: &mut heap_runtime_sites,
            structural_runtime_sites: &mut structural_runtime_sites,
            source_map: &mut source_map,
            trap_map: &mut trap_map,
            outcome_map: &mut outcome_map,
            runtime_calls: &mut runtime_call_set,
            fixups: Vec::new(),
            block_offsets: vec![None; function.blocks.len()],
            trap_offsets: [None; 3],
            status_return_offset: None,
            unregistered_status_return_offset: None,
            certified_call_roots,
            frame_bytes,
            maximum_code_bytes: plan.limits.max_code_bytes(),
        };
        encoder.emit_function()?;
        let end = encoder.bytes.len();
        let start_u32 = to_u32(start)?;
        let end_u32 = to_u32(end)?;
        entries.push(entry_metadata(
            function.id,
            function.source_function,
            function.signature.clone(),
            start_u32,
            end_u32,
        ));
        frames.push(frame_facts(
            function.id,
            frame_bytes,
            to_u32(function.values.len())?,
            to_u32(function.locals.len())?,
            outgoing_arguments,
            build_frame_homes(function)?,
            returned_structural_owner_homes(function),
        ));
        source_map.push(source_map_entry(function.id, start_u32, end_u32, None));
    }

    let mut runtime_calls: Vec<_> = runtime_call_set.into_iter().collect();
    runtime_calls.sort_by_key(|slot| match slot {
        RuntimeCallSlot::IdentityI64 => 1_u8,
        RuntimeCallSlot::Poll => 2_u8,
        RuntimeCallSlot::EnterFunction => 3_u8,
        RuntimeCallSlot::StdinHandle => 4_u8,
        RuntimeCallSlot::ByteVectorNew => 5_u8,
        RuntimeCallSlot::ByteVectorMove => 6_u8,
        RuntimeCallSlot::ByteVectorBorrowShared => 7_u8,
        RuntimeCallSlot::ByteVectorBorrowExclusive => 8_u8,
        RuntimeCallSlot::ByteSliceLength => 9_u8,
        RuntimeCallSlot::ByteSliceByteAt => 10_u8,
        RuntimeCallSlot::ByteSliceMutSetByte => 11_u8,
        RuntimeCallSlot::ByteSliceEnd => 12_u8,
        RuntimeCallSlot::ByteSliceMutEnd => 13_u8,
        RuntimeCallSlot::ByteVectorDrop => 14_u8,
        RuntimeCallSlot::ByteSliceReadU32Le => 15_u8,
        RuntimeCallSlot::ByteSliceMutWriteU32Le => 16_u8,
        RuntimeCallSlot::StaticBytesLength => 17_u8,
        RuntimeCallSlot::StaticBytesByteAt => 18_u8,
        RuntimeCallSlot::StaticBytesClone => 19_u8,
        RuntimeCallSlot::StaticBytesCopySlice => 20_u8,
        RuntimeCallSlot::StaticBytesThaw => 21_u8,
        RuntimeCallSlot::BytesMove => 22_u8,
        RuntimeCallSlot::BytesBorrowShared => 23_u8,
        RuntimeCallSlot::BytesLength => 24_u8,
        RuntimeCallSlot::BytesByteAt => 25_u8,
        RuntimeCallSlot::BytesClone => 26_u8,
        RuntimeCallSlot::BytesCopySlice => 27_u8,
        RuntimeCallSlot::BytesEndBorrow => 28_u8,
        RuntimeCallSlot::BytesDrop => 29_u8,
        RuntimeCallSlot::FreezeByteVector => 30_u8,
        RuntimeCallSlot::ThawBytes => 31_u8,
        RuntimeCallSlot::CollectReference => 32_u8,
        RuntimeCallSlot::HeapDispatch => 33_u8,
        RuntimeCallSlot::StructuralDispatch => 34_u8,
        RuntimeCallSlot::ReserveFrame => 35_u8,
        RuntimeCallSlot::RegisterFrame => 36_u8,
        RuntimeCallSlot::PublishSafepoint => 37_u8,
        RuntimeCallSlot::UnregisterFrame => 38_u8,
        RuntimeCallSlot::TakeRejectedEntry => 39_u8,
    });

    let image = InstallableImage::new(ImageParts {
        bytes,
        static_bytes: plan.static_bytes,
        entries,
        relocations,
        runtime_calls,
        execution_domain,
        frames,
        safepoints,
        root_requirements,
        heap_runtime_sites,
        structural_runtime_sites,
        source_map,
        trap_map,
        outcome_map,
        work_units: plan.work_units,
        contracts: config.contracts,
    })
    .map_err(NativeError::Image)?;
    if image.accounting().metadata_bytes() > plan.limits.max_metadata_bytes() {
        return Err(NativeError::Encode(EncodeError::LimitExceeded(
            "metadata bytes",
        )));
    }
    Ok(image)
}

fn execution_domain(functions: &[FunctionPlan]) -> NativeExecutionDomain {
    let uses_collector = functions.iter().any(|function| {
        function
            .values
            .iter()
            .any(|value| matches!(value.value_type, ValueType::Reference(_)))
            || function
                .locals
                .iter()
                .any(|local| matches!(local.value_type, ValueType::Reference(_)))
            || function.blocks.iter().any(|block| {
                block.instructions.iter().any(|instruction| {
                    matches!(instruction.operation, Operation::HeapCall(_, _))
                        || matches!(
                            instruction.operation,
                            Operation::RuntimeCall(slot, _) if slot.may_collect()
                        )
                })
            })
    });
    if uses_collector {
        NativeExecutionDomain::LegacyHeap
    } else {
        NativeExecutionDomain::CollectorFree
    }
}
