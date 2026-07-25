use super::*;

pub(super) fn collecting_function_closure(functions: &[FunctionPlan]) -> HashSet<FunctionId> {
    let mut collecting: HashSet<FunctionId> = functions
        .iter()
        .filter(|function| {
            function.blocks.iter().any(|block| {
                block.instructions.iter().any(|instruction| {
                    matches!(&instruction.operation, Operation::HeapCall(_, _))
                        || matches!(
                            &instruction.operation,
                            Operation::RuntimeCall(slot, _) if slot.may_collect()
                        )
                })
            })
        })
        .map(|function| function.id)
        .collect();
    loop {
        let mut changed = false;
        for function in functions {
            if collecting.contains(&function.id) {
                continue;
            }
            let calls_collector = function.blocks.iter().any(|block| {
                block.instructions.iter().any(|instruction| {
                    matches!(
                        &instruction.operation,
                        Operation::Call(callee, _) if collecting.contains(callee)
                    )
                })
            });
            if calls_collector {
                changed |= collecting.insert(function.id);
            }
        }
        if !changed {
            return collecting;
        }
    }
}

pub(super) fn certified_root_locations(
    function: &FunctionPlan,
    certificate: &[CertifiedRoot],
) -> Result<Vec<RootLocation>, NativeError> {
    let mut roots = Vec::new();
    roots
        .try_reserve_exact(certificate.len())
        .map_err(|_| NativeError::Encode(EncodeError::LimitExceeded("stack-map roots")))?;
    for root in certificate {
        let displacement = match root.kind {
            FrameHomeKind::Local(index) => {
                let fact = function
                    .locals
                    .get(index as usize)
                    .ok_or(NativeError::Encode(EncodeError::InvalidValue))?;
                if fact.value_type.reference_type() != Some(root.reference_type) {
                    return Err(NativeError::Encode(EncodeError::InvalidValue));
                }
                local_home_offset(index as usize)?
            }
            FrameHomeKind::Value(index) => {
                let fact = function
                    .values
                    .get(index as usize)
                    .ok_or(NativeError::Encode(EncodeError::InvalidValue))?;
                if fact.value_type.reference_type() != Some(root.reference_type) {
                    return Err(NativeError::Encode(EncodeError::InvalidValue));
                }
                value_home_offset(function, index as usize)?
            }
        };
        roots.push(root_location(displacement, root.kind, root.reference_type));
    }
    roots.sort_unstable();
    if roots.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(NativeError::Encode(EncodeError::InvalidValue));
    }
    Ok(roots)
}
