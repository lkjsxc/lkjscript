pub(in crate::run) fn call_memory_witnesses<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    callee_index: u32,
    proto: &lkjscript_core::FunctionProto,
    arguments: &[Value],
    call_offset: usize,
) -> Result<Vec<lkjscript_core::MemoryWitnessBinding>> {
    let caller_frame = vm
        .frames
        .last()
        .ok_or_else(|| Error::msg("generic call has no caller frame"))?;
    let caller = if caller_frame.proto == u32::MAX {
        vm.chunk.main()
    } else {
        vm.chunk
            .protos()
            .get(caller_frame.proto as usize)
            .ok_or_else(|| Error::msg("generic call caller metadata is missing"))?
    };
    let offset = u64::try_from(call_offset)
        .map_err(|_| Error::msg("generic call offset exceeds u64"))?;
    let site = caller
        .call_witnesses
        .binary_search_by_key(&offset, |site| site.offset)
        .ok()
        .and_then(|index| caller.call_witnesses.get(index));
    if proto.memory_witness_parameters.is_empty() {
        return Ok(Vec::new());
    }
    let site = site.ok_or_else(|| Error::msg("generic call witness site is missing"))?;
    if site.callee != callee_index || site.bindings.len() != proto.memory_witness_parameters.len() {
        return Err(Error::msg("generic call witness signature is stale"));
    }
    for (requirement, binding) in proto.memory_witness_parameters.iter().zip(&site.bindings) {
        let witness = vm
            .chunk
            .memory_witnesses()
            .get(usize::from(binding.witness))
            .ok_or_else(|| Error::msg("generic call witness slot is invalid"))?;
        if binding.parameter != requirement.parameter {
            return Err(Error::msg("generic call witness parameter is stale"));
        }
        for (index, variable) in proto.parameter_type_variables.iter().enumerate() {
            if *variable != Some(requirement.parameter) {
                continue;
            }
            let value = arguments
                .get(index)
                .copied()
                .ok_or_else(|| Error::msg("generic call witness argument is missing"))?;
            if !runtime_witness_matches(vm, witness.value_kind, value)? {
                return Err(Error::msg("generic call witness does not match runtime argument"));
            }
        }
    }
    Ok(site.bindings.clone())
}

fn runtime_witness_matches<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    kind: lkjscript_core::MemoryWitnessValueKind,
    value: Value,
) -> Result<bool> {
    Ok(match kind {
        lkjscript_core::MemoryWitnessValueKind::Unit => value.is_unit(),
        lkjscript_core::MemoryWitnessValueKind::Bool => value.as_bool().is_some(),
        lkjscript_core::MemoryWitnessValueKind::I64 => value.as_i64().is_some(),
        lkjscript_core::MemoryWitnessValueKind::F64 => value.as_f64_bits().is_some(),
        lkjscript_core::MemoryWitnessValueKind::List => value.as_segmented_list().is_some(),
        lkjscript_core::MemoryWitnessValueKind::Structural(expected) => {
            value.as_structural_root().is_some()
                && same_representation_type(
                    vm.chunk,
                    invocation(vm)?.owner(value)?.1.representation,
                    expected,
                )?
        }
        lkjscript_core::MemoryWitnessValueKind::Unsupported => false,
    })
}
