use super::*;

fn place(ty: SsaType, drop_glue: Option<DropGlueIdentity>) -> PlaceMetadata {
    PlaceMetadata {
        id: PlaceId::new(0),
        binding: BindingId::new(0),
        ty,
        drop_glue,
    }
}

#[test]
fn direct_affine_memory_modes_are_closed() {
    let owned = owner_mode(&place(
        SsaType::ByteVector,
        Some(DropGlueIdentity::ByteVector),
    ))
    .unwrap_or_else(|| unreachable!());
    assert_eq!(owned.storage, MemoryStorage::DeterministicUnique);
    assert_eq!(
        owned.destruction,
        MemoryDestruction::DropGlue(DropGlueIdentity::ByteVector)
    );
    let resource = owner_mode(&place(
        SsaType::Resource(lkjscript_contracts::ResourceKind::FileReader),
        Some(DropGlueIdentity::Resource(
            lkjscript_contracts::ResourceKind::FileReader,
        )),
    ))
    .unwrap_or_else(|| unreachable!());
    assert_eq!(resource.storage, MemoryStorage::ExternalSlot);
    assert_eq!(
        resource.destruction,
        MemoryDestruction::DropGlue(DropGlueIdentity::Resource(
            lkjscript_contracts::ResourceKind::FileReader
        ))
    );
    assert!(owner_mode(&place(SsaType::Buf, None)).is_none());
}
