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
    let mut source_map = Vec::new();
    let mut trap_map = Vec::new();
    let mut outcome_map = Vec::new();
    let signatures: Vec<_> = plan
        .functions
        .iter()
        .map(|function| (function.id, function.signature.clone()))
        .collect();
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
            collecting_functions: &collecting_functions,
            bytes: &mut bytes,
            relocations: &mut relocations,
            safepoints: &mut safepoints,
            root_requirements: &mut root_requirements,
            heap_runtime_sites: &mut heap_runtime_sites,
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
        ));
        source_map.push(source_map_entry(function.id, start_u32, end_u32, None));
    }

    let mut runtime_calls: Vec<_> = runtime_call_set.into_iter().collect();
    runtime_calls.sort_by_key(|slot| match slot {
        RuntimeCallSlot::IdentityI64V1 => 1_u8,
        RuntimeCallSlot::PollV1 => 2_u8,
        RuntimeCallSlot::EnterFunctionV1 => 3_u8,
        RuntimeCallSlot::CollectReferenceV1 => 4_u8,
        RuntimeCallSlot::HeapDispatchV1 => 5_u8,
        RuntimeCallSlot::ReserveFrameV1 => 6_u8,
        RuntimeCallSlot::RegisterFrameV1 => 7_u8,
        RuntimeCallSlot::PublishSafepointV1 => 8_u8,
        RuntimeCallSlot::UnregisterFrameV1 => 9_u8,
    });

    let image = InstallableImage::new(ImageParts {
        bytes,
        entries,
        relocations,
        runtime_calls,
        frames,
        safepoints,
        root_requirements,
        heap_runtime_sites,
        source_map,
        trap_map,
        outcome_map,
        work_units: plan.work_units,
        versions: config.versions,
    })
    .map_err(NativeError::Image)?;
    if image.accounting().metadata_bytes() > plan.limits.max_metadata_bytes() {
        return Err(NativeError::Encode(EncodeError::LimitExceeded(
            "metadata bytes",
        )));
    }
    Ok(image)
}
