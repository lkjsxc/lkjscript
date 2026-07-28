fn owner_mode(place: &PlaceMetadata) -> Option<MemoryMode> {
    let glue = place.drop_glue?;
    let (storage, destruction, identity) = match (&place.ty, glue) {
        (SsaType::Owned(inner), DropGlueIdentity::LegacyTracedByteVector)
            if **inner == SsaType::Buf =>
        {
            (
                MemoryStorage::TransitionalTracedBuffer,
                MemoryDestruction::DropGlue(glue),
                MemoryIdentity::Value,
            )
        }
        (SsaType::Resource(kind), DropGlueIdentity::Resource(glue_kind)) if *kind == glue_kind => (
            MemoryStorage::ExternalSlot,
            MemoryDestruction::ExplicitExternalClose,
            MemoryIdentity::External,
        ),
        _ => return None,
    };
    Some(MemoryMode {
        multiplicity: MemoryMultiplicity::Affine,
        aliasing: MemoryAliasing::Unique,
        locality: MemoryLocality::LocalOrEscaping,
        storage,
        portability: MemoryPortability::WorkerLocal,
        contention: MemoryContention::SingleOwner,
        destruction,
        identity,
    })
}

fn borrow_mode(kind: BorrowKind) -> MemoryMode {
    MemoryMode {
        multiplicity: match kind {
            BorrowKind::Shared => MemoryMultiplicity::Copy,
            BorrowKind::Mutable => MemoryMultiplicity::Affine,
        },
        aliasing: match kind {
            BorrowKind::Shared => MemoryAliasing::BorrowedShared,
            BorrowKind::Mutable => MemoryAliasing::BorrowedExclusive,
        },
        locality: MemoryLocality::BorrowLocal,
        storage: MemoryStorage::BorrowedView,
        portability: MemoryPortability::WorkerLocal,
        contention: MemoryContention::SingleOwner,
        destruction: MemoryDestruction::EndBorrow,
        identity: MemoryIdentity::Value,
    }
}

fn sort_key(obligation: &SsaMemoryObligation) -> (u32, u8, u32, u32, u32) {
    match obligation.subject {
        MemoryObligationSubject::Owner { place, binding } => {
            (obligation.function.raw(), 0, place.raw(), binding.raw(), 0)
        }
        MemoryObligationSubject::Loan { place, loan, value } => (
            obligation.function.raw(),
            1,
            place.raw(),
            loan.raw(),
            value.raw(),
        ),
    }
}
