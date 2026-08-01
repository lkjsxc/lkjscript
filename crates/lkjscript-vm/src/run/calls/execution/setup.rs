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
    vm.chunk
        .protos()
        .get(frame.proto as usize)
        .and_then(|proto| proto.code.get(frame.ip))
        .copied()
        == Some(Op::Return as u8)
}
