use super::*;

pub(crate) fn verified_fold(
    children: Vec<(VerifiedDerived, MemoryTypePathElement)>,
    region_capable: bool,
) -> VerifiedDerived {
    let mut mode = MemoryAggregateMode::Copy;
    let mut borrow = false;
    let mut dynamic = false;
    let mut region = None;
    let mut blocker = None;
    for (child, path) in children {
        mode = mode.max(child.mode);
        borrow |= child.contains_borrow;
        dynamic |= child.contains_dynamic_owner;
        match child.closure.class {
            MemoryClosureClass::Deterministic => {}
            MemoryClosureClass::RegionClosed if region.is_none() => {
                region = Some((child.closure, path));
            }
            MemoryClosureClass::RegionClosed => {}
            MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge
                if blocker.is_none() =>
            {
                blocker = Some((child.closure, path));
            }
            MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge => {}
        }
    }
    if let Some((mut closure, path)) = blocker {
        closure.blocker_path.insert(0, path);
        if dynamic {
            closure.class = MemoryClosureClass::IllegalDomainBridge;
            closure.mixed_direction =
                Some(MemoryMixedBridgeDirection::DeterministicContainsUnresolved);
        }
        return VerifiedDerived {
            mode,
            closure,
            contains_borrow: borrow,
            contains_dynamic_owner: dynamic,
        };
    }
    if let Some((mut closure, path)) = region {
        closure.blocker_path.insert(0, path);
        if dynamic || borrow {
            closure.class = MemoryClosureClass::IllegalDomainBridge;
            closure.mixed_direction =
                Some(MemoryMixedBridgeDirection::DeterministicContainsUnresolved);
        } else if !region_capable {
            closure.class = MemoryClosureClass::Unresolved;
        }
        return VerifiedDerived {
            mode,
            closure,
            contains_borrow: borrow,
            contains_dynamic_owner: dynamic,
        };
    }
    VerifiedDerived {
        mode,
        closure: verified_closed(MemoryClosureClass::Deterministic),
        contains_borrow: borrow,
        contains_dynamic_owner: true,
    }
}

pub(crate) fn verified_closed(class: MemoryClosureClass) -> MemoryClosureFact {
    MemoryClosureFact {
        class,
        blocker_path: Vec::new(),
        blocker_type: None,
        blocker_reason: None,
        mixed_direction: None,
    }
}

pub(crate) fn verified_unresolved(ty: &Type, reason: MemoryBlockerReason) -> VerifiedDerived {
    VerifiedDerived {
        mode: MemoryAggregateMode::ImmutableValue,
        closure: MemoryClosureFact {
            class: MemoryClosureClass::Unresolved,
            blocker_path: Vec::new(),
            blocker_type: Some(verified_memory_type(ty)),
            blocker_reason: Some(reason),
            mixed_direction: None,
        },
        contains_borrow: false,
        contains_dynamic_owner: false,
    }
}

pub(crate) fn verified_type_contains_resource(ty: &Type) -> bool {
    match ty {
        Type::Resource(_) => true,
        Type::List(inner) => verified_type_contains_resource(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(verified_type_contains_resource),
        Type::Fn { params, ret } => {
            params.iter().any(verified_type_contains_resource)
                || verified_type_contains_resource(ret)
        }
        Type::Forall { body, .. } => verified_type_contains_resource(body),
        _ => false,
    }
}

pub(crate) fn verified_key(ty: &Type) -> Option<VerifiedDeclarationKey> {
    match ty {
        Type::Product(id) => Some(VerifiedDeclarationKey::Product(*id)),
        Type::Enum { id, .. } => Some(VerifiedDeclarationKey::Enum(id.bytes())),
        _ => None,
    }
}

pub(crate) fn verified_is_aggregate(ty: &Type) -> bool {
    matches!(ty, Type::Product(_) | Type::Enum { .. })
}

pub(crate) fn verified_copy_share(ty: &Type, item: &VerifiedDerived) -> MemoryCopySharePlan {
    if item.closure.class == MemoryClosureClass::RegionClosed {
        return MemoryCopySharePlan::RegionHandleCopy;
    }
    if item.closure.class != MemoryClosureClass::Deterministic {
        return MemoryCopySharePlan::Unsupported;
    }
    match ty {
        Type::Symbol => MemoryCopySharePlan::StaticIdentity,
        Type::ByteSlice => MemoryCopySharePlan::BorrowShared,
        Type::ByteSliceMut => MemoryCopySharePlan::BorrowExclusive,
        Type::Resource(_) => MemoryCopySharePlan::ExternalHandle,
        Type::List(_) => MemoryCopySharePlan::RegionHandleCopy,
        Type::Product(_) | Type::Enum { .. } => MemoryCopySharePlan::StructuralCopy,
        _ => match item.mode {
            MemoryAggregateMode::Copy => MemoryCopySharePlan::TrivialCopy,
            MemoryAggregateMode::ImmutableValue if verified_is_aggregate(ty) => {
                MemoryCopySharePlan::StructuralCopy
            }
            MemoryAggregateMode::ImmutableValue => MemoryCopySharePlan::BorrowShared,
            MemoryAggregateMode::Affine => MemoryCopySharePlan::Move,
        },
    }
}

pub(crate) fn verified_leaf_glue(ty: &Type) -> Option<MemoryDropGlueId> {
    match ty {
        Type::ByteVector => Some(MemoryDropGlueId::new(0)),
        Type::Bytes => Some(MemoryDropGlueId::new(1 + ResourceKind::ALL.len() as u64)),
        Type::Resource(kind) => Some(MemoryDropGlueId::new(1 + *kind as u64)),
        _ => None,
    }
}

pub(crate) fn verified_observe(slot: &mut u64, amount: usize) -> Result<()> {
    *slot = slot
        .checked_add(
            u64::try_from(amount).map_err(|_| Error::msg("memory verifier work exceeds u64"))?,
        )
        .ok_or_else(|| Error::msg("memory verifier aggregate telemetry overflow"))?;
    Ok(())
}
