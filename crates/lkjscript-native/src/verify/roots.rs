use super::*;

pub(super) const ROOT_RECORD_METADATA_BYTES: u64 = 32;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum LiveHome {
    Local(u32),
    Value(u32),
}

pub(super) fn derive_call_root_requirements(
    function: &FunctionPlan,
    work: &mut u64,
    maximum_work: u64,
    root_records: &mut u64,
    maximum_root_records: u64,
) -> Result<FunctionRootRequirements, VerificationError> {
    let mut live_in = vec![BTreeSet::new(); function.blocks.len()];
    loop {
        let mut changed = false;
        for block_index_value in (0..function.blocks.len()).rev() {
            charge_liveness(work, maximum_work)?;
            let block = &function.blocks[block_index_value];
            let mut live = successor_reference_live(block, &live_in, work, maximum_work)?;
            add_terminator_references(function, block, &mut live, work, maximum_work)?;
            for instruction in block.instructions.iter().rev() {
                transfer_reference_liveness(function, instruction, &mut live, work, maximum_work)?;
                charge_liveness(work, maximum_work)?;
            }
            if live != live_in[block_index_value] {
                live_in[block_index_value] = live;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut call_roots = Vec::new();
    call_roots
        .try_reserve_exact(function.values.len())
        .map_err(|_| VerificationError::LimitExceeded("stack-map certificate storage"))?;
    call_roots.resize_with(function.values.len(), || None);
    for block in &function.blocks {
        let mut live = successor_reference_live(block, &live_in, work, maximum_work)?;
        add_terminator_references(function, block, &mut live, work, maximum_work)?;
        for instruction in block.instructions.iter().rev() {
            if matches!(
                &instruction.operation,
                Operation::Call(_, _) | Operation::RuntimeCall(_, _) | Operation::HeapCall(_, _)
            ) {
                let output_home = LiveHome::Value(instruction.output.index);
                let mut roots = Vec::new();
                for home in live.iter().copied().filter(|home| *home != output_home) {
                    push_certified_root(
                        function,
                        home,
                        &mut roots,
                        work,
                        maximum_work,
                        root_records,
                        maximum_root_records,
                    )?;
                }
                for operand in instruction.operation.operands() {
                    if value_reference_type(function, operand).is_none() {
                        continue;
                    }
                    let home = LiveHome::Value(operand.index);
                    if home != output_home && live.contains(&home) {
                        continue;
                    }
                    let kind = crate::FrameHomeKind::Value(operand.index);
                    if roots.iter().any(|root| root.kind == kind) {
                        continue;
                    }
                    push_certified_root(
                        function,
                        home,
                        &mut roots,
                        work,
                        maximum_work,
                        root_records,
                        maximum_root_records,
                    )?;
                }
                roots.sort_unstable();
                let slot = call_roots
                    .get_mut(instruction.output.index as usize)
                    .ok_or(VerificationError::InvalidValue(instruction.output))?;
                *slot = Some(roots);
            }
            transfer_reference_liveness(function, instruction, &mut live, work, maximum_work)?;
            charge_liveness(work, maximum_work)?;
        }
    }
    Ok(call_roots)
}
