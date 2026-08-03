fn independent_owners(
    facts: &[PlacementFact<'_>],
    function: MemoryFunctionId,
    binding: BindingId,
) -> u32 {
    facts
        .iter()
        .filter(|parent| parent.function == function)
        .filter(|parent| matches!(parent.expression.kind, ExprKind::Call { .. }))
        .map(|parent| {
            facts
                .iter()
                .filter(|child| child.parent == Some(parent.id))
                .filter(|child| {
                    matches!(
                        &child.expression.kind,
                        ExprKind::Load(reference) if reference.binding == binding
                    )
                })
                .count()
        })
        .max()
        .and_then(|count| u32::try_from(count).ok())
        .unwrap_or(1)
        .max(1)
}

fn diverges_across_branch(
    facts: &[PlacementFact<'_>],
    function: MemoryFunctionId,
    binding: BindingId,
) -> bool {
    let mut branches = std::collections::BTreeSet::new();
    for fact in facts.iter().filter(|fact| fact.function == function) {
        if !matches!(
            &fact.expression.kind,
            ExprKind::Load(reference) | ExprKind::Move { binding: reference, .. }
                if reference.binding == binding
        ) {
            continue;
        }
        if let Some(branch) = enclosing_if_branch(facts, fact) {
            branches.insert(branch);
        }
    }
    branches.contains(&1) && branches.contains(&2)
}

fn enclosing_if_branch(
    facts: &[PlacementFact<'_>],
    fact: &PlacementFact<'_>,
) -> Option<u32> {
    let mut child = fact;
    while let Some(parent_id) = child.parent {
        let parent = facts.iter().find(|candidate| candidate.id == parent_id)?;
        if matches!(parent.expression.kind, ExprKind::If { .. }) {
            return Some(child.child_index);
        }
        child = parent;
    }
    None
}

fn sealed_selected(
    witness: &MemoryWitness,
    independent: u32,
    nodes: u64,
    bytes: u64,
    dependencies: usize,
    release_cost: u64,
) -> bool {
    let facts = &witness.facts;
    facts.mode == MemoryAggregateMode::ImmutableValue
        && facts.capabilities.sealed_region
        && facts.capabilities.process_codec
        && facts.process_codec == MemoryProcessCodecEligibility::Eligible
        && !facts.contains_borrow
        && !memory_type_has_affine_or_resource(&facts.ty)
        && independent >= 2
        && (nodes >= INITIAL_SEAL_NODES || bytes >= INITIAL_SEAL_BYTES)
        && dependencies <= MAX_SEAL_DEPENDENCIES
        && release_cost <= MAX_SEAL_RELEASE_WORK
}

fn memory_type_has_affine_or_resource(ty: &MemoryType) -> bool {
    match ty {
        MemoryType::Resource(_) | MemoryType::ByteVector | MemoryType::ByteSliceMut => true,
        MemoryType::List(inner) => memory_type_has_affine_or_resource(inner),
        MemoryType::Enum { arguments, .. } => arguments.iter().any(memory_type_has_affine_or_resource),
        MemoryType::Function { parameters, result } => {
            parameters.iter().any(memory_type_has_affine_or_resource)
                || memory_type_has_affine_or_resource(result)
        }
        MemoryType::ForAll { body, .. } => memory_type_has_affine_or_resource(body),
        _ => false,
    }
}

fn select_route(
    fact: &PlacementFact<'_>,
    last_use: bool,
    independent: u32,
    nodes: u64,
    bytes: u64,
    sealed: bool,
) -> MemoryValueRoute {
    if matches!(fact.expression.kind, ExprKind::Borrow { .. } | ExprKind::BorrowBytes { .. }) {
        MemoryValueRoute::Borrow
    } else if last_use || matches!(fact.expression.kind, ExprKind::Move { .. }) {
        MemoryValueRoute::LastUseMove
    } else if matches!(fact.expression.kind, ExprKind::WithProductField { .. }) {
        MemoryValueRoute::UniqueReuse
    } else if independent >= 2 && nodes < INITIAL_SEAL_NODES && bytes < INITIAL_SEAL_BYTES {
        MemoryValueRoute::DetachedClone
    } else if sealed {
        MemoryValueRoute::SealedShare
    } else if matches!(fact.expression.kind, ExprKind::Load(_)) {
        MemoryValueRoute::DetachedClone
    } else {
        MemoryValueRoute::UniqueReuse
    }
}

fn selected_storage(route: MemoryValueRoute, sealed: bool) -> MemoryDomain {
    if route == MemoryValueRoute::Borrow {
        MemoryDomain::BorrowedView
    } else if sealed {
        MemoryDomain::SealedRegion
    } else {
        MemoryDomain::UniqueStructural
    }
}

fn cleanup(route: MemoryValueRoute, storage: MemoryDomain) -> MemoryValueFailureCleanup {
    match (route, storage) {
        (MemoryValueRoute::Borrow, _) => MemoryValueFailureCleanup::EndBorrow,
        (_, MemoryDomain::SealedRegion) => MemoryValueFailureCleanup::DisposeSealedOwner,
        (_, MemoryDomain::UniqueStructural) => MemoryValueFailureCleanup::DisposeUniqueOwner,
        _ => MemoryValueFailureCleanup::None,
    }
}

fn representation_id(
    type_fact: &MemoryTypeFact,
    category: MemoryValueCategory,
    storage: MemoryDomain,
    route: MemoryValueRoute,
) -> Result<MemoryValueRepresentationId> {
    let mut bytes = b"lkjscript.memory-value-representation\0canonical-platform-contract".to_vec();
    bytes.extend_from_slice(&type_fact.witness.as_bytes());
    let _ = route;
    bytes.extend_from_slice(&[category_tag(category), storage_tag(storage)]);
    Ok(MemoryValueRepresentationId::from_bytes(lkjscript_core::sha256(&bytes)))
}

fn category_tag(value: MemoryValueCategory) -> u8 {
    match value {
        MemoryValueCategory::Owner => 0,
        MemoryValueCategory::View => 1,
        MemoryValueCategory::Destination => 2,
    }
}
fn storage_tag(value: MemoryDomain) -> u8 {
    match value {
        MemoryDomain::Inline => 0, MemoryDomain::Static => 1, MemoryDomain::Stack => 2,
        MemoryDomain::CallerDestination => 3, MemoryDomain::UniqueStructural => 4,
        MemoryDomain::OrdinaryRegion => 5, MemoryDomain::SealedRegion => 6,
        MemoryDomain::BorrowedView => 7, MemoryDomain::ExternalResource => 8,
        MemoryDomain::UnsupportedRuntime => 9,
    }
}
