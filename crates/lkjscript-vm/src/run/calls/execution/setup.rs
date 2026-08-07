fn initial_unique_places(
    proto: &lkjscript_core::FunctionProto,
    arguments: &[Value],
) -> Result<Vec<crate::run::unique::RuntimePlace>> {
    let mut places = Vec::new();
    places
        .try_reserve_exact(proto.unique_places)
        .map_err(|_| Error::host("VM call unique-place reservation failed"))?;
    places.resize(
        proto.unique_places,
        crate::run::unique::RuntimePlace::Inactive,
    );
    for (index, place) in proto.parameter_unique_places.iter().copied().enumerate() {
        let Some(place) = place else {
            continue;
        };
        let kind = proto
            .parameter_uniques
            .get(index)
            .copied()
            .flatten()
            .ok_or_else(|| Error::msg("owner-place parameter lacks unique kind metadata"))?;
        let owner = arguments
            .get(index)
            .and_then(|value| match kind {
                lkjscript_core::UniqueValueKind::Bytes => value.as_bytes_key(),
                lkjscript_core::UniqueValueKind::ByteVector => value.as_byte_vector_key(),
                _ => None,
            })
            .ok_or_else(|| Error::msg("call parameter lacks exact unique owner payload"))?;
        let target = places
            .get_mut(place)
            .ok_or_else(|| Error::msg("byte-vector call parameter PlaceId is out of range"))?;
        if !matches!(target, crate::run::unique::RuntimePlace::Inactive) {
            return Err(Error::msg("duplicate byte-vector call parameter PlaceId"));
        }
        *target = crate::run::unique::RuntimePlace::Active {
            owner: Some(owner),
            transferred: None,
        };
    }
    Ok(places)
}

fn is_tail_position(vm: &Vm<'_>) -> bool {
    let Some(frame) = vm.frames.last() else {
        return false;
    };
    let Some(prototype) = frame.proto else {
        return false;
    };
    let Some(code) = vm
        .chunk
        .protos()
        .get(prototype)
        .map(|proto| proto.code.as_slice())
    else {
        return false;
    };
    if code.get(frame.ip).copied() == Some(Op::Return as u8) {
        return true;
    }
    forwarding_epilogue(code, frame.ip)
}

fn forwarding_epilogue(code: &[u8], mut ip: usize) -> bool {
    let Some((load, slot, next)) = forwarding_store(code, ip) else {
        return false;
    };
    ip = next;
    loop {
        if decode_index_instruction(code, ip)
            .is_some_and(|(op, actual, next)| {
                op == load
                    && actual == slot
                    && code.get(next).copied() == Some(Op::Return as u8)
            })
        {
            return true;
        }
        let Some(op) = code.get(ip).and_then(|byte| Op::from_byte(*byte)) else {
            return false;
        };
        if !matches!(
            op,
            Op::ByteVectorPlaceEnd | Op::BytesPlaceEnd | Op::StructuralPlaceEnd
        ) {
            return false;
        }
        let Some(next) = ip
            .checked_add(1)
            .and_then(|operand| operand.checked_add(op.operand_width()))
            .filter(|next| *next <= code.len())
        else {
            return false;
        };
        ip = next;
        let Some((store, _, next)) = decode_index_instruction(code, ip) else {
            return false;
        };
        if store != Op::StoreLocal || code.get(next).copied() != Some(Op::Pop as u8) {
            return false;
        }
        let Some(next) = next.checked_add(1) else {
            return false;
        };
        ip = next;
    }
}

fn forwarding_store(code: &[u8], ip: usize) -> Option<(Op, usize, usize)> {
    let (op, slot, next) = decode_index_instruction(code, ip)?;
    match op {
        Op::StoreUniqueLocal => Some((Op::TakeUniqueLocal, slot, next)),
        Op::StoreStructuralLocal => Some((Op::TakeStructuralLocal, slot, next)),
        Op::StoreLocal if code.get(next).copied() == Some(Op::Pop as u8) => {
            Some((Op::LoadLocal, slot, next.checked_add(1)?))
        }
        _ => None,
    }
}

fn decode_index_instruction(code: &[u8], ip: usize) -> Option<(Op, usize, usize)> {
    let op = code.get(ip).and_then(|byte| Op::from_byte(*byte))?;
    if op.operand_layout() != lkjscript_core::OperandLayout::Index {
        return None;
    }
    let operand = ip.checked_add(1)?;
    let next = operand.checked_add(op.operand_width())?;
    let bytes: [u8; 8] = code.get(operand..next)?.try_into().ok()?;
    let index = usize::try_from(u64::from_le_bytes(bytes)).ok()?;
    Some((op, index, next))
}
