fn initial_locals(chunk: &Chunk, proto: &FunctionProto, is_main: bool) -> Vec<Option<Kind>> {
    let mut locals = vec![None; usize::from(proto.locals)];
    for (index, slot) in locals.iter_mut().take(usize::from(proto.arity)).enumerate() {
        let resource = proto
            .parameter_resources
            .get(index)
            .copied()
            .flatten()
            .map(|kind| Kind::Resource {
                kind,
                owner: if proto
                    .parameter_resource_places
                    .get(index)
                    .copied()
                    .flatten()
                    .is_some()
                {
                    0xb000_0000 | u32::try_from(index).unwrap_or(u32::MAX)
                } else {
                    0
                },
            });
        let unique = proto
            .parameter_uniques
            .get(index)
            .copied()
            .flatten()
            .map(|kind| match kind {
                crate::UniqueValueKind::Bytes => {
                    Kind::Bytes(0x8000_0000 | u32::try_from(index).unwrap_or(u32::MAX))
                }
                crate::UniqueValueKind::ByteVector => {
                    Kind::ByteVector(0x8000_0000 | u32::try_from(index).unwrap_or(u32::MAX))
                }
                crate::UniqueValueKind::ByteSlice => Kind::ByteSlice {
                    owner: 0x9000_0000 | u32::try_from(index).unwrap_or(u32::MAX),
                    mutable: false,
                    used: false,
                },
                crate::UniqueValueKind::ByteSliceMut => Kind::ByteSlice {
                    owner: 0x9000_0000 | u32::try_from(index).unwrap_or(u32::MAX),
                    mutable: true,
                    used: false,
                },
            });
        let structural = proto
            .parameter_structurals
            .get(index)
            .copied()
            .flatten()
            .map(|representation| {
                let owner = 0xa000_0000 | u32::try_from(index).unwrap_or(u32::MAX);
                if proto
                    .parameter_structural_places
                    .get(index)
                    .copied()
                    .flatten()
                    .is_some()
                {
                    Kind::StructuralOwner {
                        representation,
                        owner,
                        active_variant: None,
                    }
                } else {
                    Kind::StructuralOwnerRef {
                        representation,
                        owner,
                        active_variant: None,
                    }
                }
            });
        *slot = resource.or(unique).or(structural).or(Some(Kind::Any));
    }
    if is_main {
        for (slot, kind) in locals.iter_mut().zip(&chunk.required_capabilities) {
            *slot = Some(Kind::Capability(*kind));
        }
    }
    locals
}
