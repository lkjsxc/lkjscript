use super::*;

#[test]
fn direct_affine_modes_are_closed_and_truthful_about_transition() {
    let owned =
        owner_mode(&SsaType::Owned(Box::new(SsaType::Buf))).unwrap_or_else(|| unreachable!());
    assert_eq!(owned.storage, MemoryStorage::TransitionalTracedBuffer);
    assert_eq!(owned.destruction, MemoryDestruction::CompilerFactOnly);
    let resource = owner_mode(&SsaType::Resource(
        lkjscript_contracts::ResourceKind::FileReader,
    ))
    .unwrap_or_else(|| unreachable!());
    assert_eq!(resource.identity, MemoryIdentity::External);
    assert_eq!(
        resource.destruction,
        MemoryDestruction::ExplicitExternalClose
    );
    assert!(owner_mode(&SsaType::Buf).is_none());
}
