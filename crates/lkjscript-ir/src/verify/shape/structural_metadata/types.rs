fn verify_type_layout(
    program: &Program,
    ty: &SsaType,
    layout: crate::StructuralLayoutId,
) -> crate::Result<()> {
    let kind = &program.memory.layouts[layout.index().unwrap_or(usize::MAX)].kind;
    let matches = match (ty, kind) {
        (SsaType::Str, StructuralLayoutKind::String)
        | (SsaType::Path, StructuralLayoutKind::Path) => true,
        (SsaType::Product(left), StructuralLayoutKind::Product { product: right, .. }) => {
            left == right
        }
        (SsaType::Enum { id: left, .. }, StructuralLayoutKind::Enum { enum_id: right, .. }) => {
            left == right
        }
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        fail("SSA structural type and layout kind do not match")
    }
}

fn storage_matches(category: StructuralValueCategory, storage: StructuralStorage) -> bool {
    matches!(
        (category, storage),
        (
            StructuralValueCategory::Owner,
            StructuralStorage::Static
                | StructuralStorage::UniqueStructural
                | StructuralStorage::OrdinaryRegion
                | StructuralStorage::SealedRegion
                | StructuralStorage::ExternalResource
        ) | (
            StructuralValueCategory::View,
            StructuralStorage::Stack | StructuralStorage::BorrowedView
        ) | (
            StructuralValueCategory::Destination,
            StructuralStorage::CallerDestination
                | StructuralStorage::UniqueStructural
                | StructuralStorage::OrdinaryRegion
                | StructuralStorage::SealedRegion
        )
    )
}
