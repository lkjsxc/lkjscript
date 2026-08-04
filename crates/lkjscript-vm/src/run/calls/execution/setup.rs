fn initial_unique_places(
    proto: &lkjscript_core::FunctionProto,
    arguments: &[Value],
) -> Result<Vec<crate::run::unique::RuntimePlace>> {
    let mut places =
        vec![crate::run::unique::RuntimePlace::Inactive; usize::from(proto.unique_places)];
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
            .get_mut(usize::from(place))
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

fn is_tail_position<J: RuntimeTier>(vm: &Vm<'_, J>) -> bool {
    let Some(frame) = vm.frames.last() else {
        return false;
    };
    if frame.proto == u32::MAX {
        return false;
    }
    let Some(code) = vm
        .chunk
        .protos()
        .get(frame.proto as usize)
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
        if code.get(ip).copied() == Some(load as u8)
            && code.get(ip.saturating_add(1)).copied() == Some(slot)
            && code.get(ip.saturating_add(2)).copied() == Some(Op::Return as u8)
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
        ip = ip.saturating_add(2);
        if code.get(ip).copied() != Some(Op::StoreLocal as u8)
            || code.get(ip.saturating_add(2)).copied() != Some(Op::Pop as u8)
        {
            return false;
        }
        ip = ip.saturating_add(3);
    }
}

fn forwarding_store(code: &[u8], ip: usize) -> Option<(Op, u8, usize)> {
    let op = code.get(ip).and_then(|byte| Op::from_byte(*byte))?;
    let slot = *code.get(ip.checked_add(1)?)?;
    match op {
        Op::StoreUniqueLocal => Some((Op::TakeUniqueLocal, slot, ip.checked_add(2)?)),
        Op::StoreStructuralLocal => Some((Op::TakeStructuralLocal, slot, ip.checked_add(2)?)),
        Op::StoreLocal if code.get(ip.checked_add(2)?).copied() == Some(Op::Pop as u8) => {
            Some((Op::LoadLocal, slot, ip.checked_add(3)?))
        }
        _ => None,
    }
}
