fn verified_index_independent_owners(
    facts: &Facts<'_>,
) -> Result<HashMap<(MemoryFunctionId, BindingId), u64>> {
    let mut call_counts: HashMap<(MemoryExpressionId, BindingId), u64> = HashMap::new();
    let mut output = HashMap::new();
    for fact in &facts.expressions {
        let hir::ExprKind::Load(reference) = &fact.expression.kind else {
            continue;
        };
        let Some(parent_id) = fact.parent else {
            continue;
        };
        let Some(parent) = facts.expression(parent_id) else {
            continue;
        };
        if !matches!(parent.expression.kind, hir::ExprKind::Call { .. }) {
            continue;
        }
        let count = call_counts
            .entry((parent_id, reference.binding))
            .or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| Error::msg("verified independent-owner count overflow"))?;
        let maximum = output
            .entry((fact.function, reference.binding))
            .or_insert(1);
        *maximum = (*maximum).max(*count);
    }
    Ok(output)
}

fn verified_index_branch_divergence(
    facts: &Facts<'_>,
) -> HashMap<(MemoryFunctionId, BindingId), bool> {
    let mut branches: HashMap<(MemoryFunctionId, BindingId), u8> = HashMap::new();
    for fact in &facts.expressions {
        let binding = match &fact.expression.kind {
            hir::ExprKind::Load(reference)
            | hir::ExprKind::Move { binding: reference, .. } => reference.binding,
            _ => continue,
        };
        let mut child = fact;
        while let Some(parent_id) = child.parent {
            let Some(parent) = facts.expression(parent_id) else {
                break;
            };
            if matches!(parent.expression.kind, hir::ExprKind::If { .. }) {
                let bit = match child.child_index {
                    1 => 1,
                    2 => 2,
                    _ => 0,
                };
                *branches.entry((fact.function, binding)).or_default() |= bit;
                break;
            }
            child = parent;
        }
    }
    branches
        .into_iter()
        .map(|(key, branches)| (key, branches == 3))
        .collect()
}

fn verified_sealed_selected(
    witness: &MemoryWitness,
    independent: u64,
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
        && !verified_affine_or_resource(&facts.ty)
        && independent >= 2
        && (nodes >= INITIAL_SEAL_NODES || bytes >= INITIAL_SEAL_BYTES)
        && dependencies <= MAX_SEAL_DEPENDENCIES
        && release_cost <= MAX_SEAL_RELEASE_WORK
}

fn verified_affine_or_resource(ty: &MemoryType) -> bool {
    match ty {
        MemoryType::Resource(_) | MemoryType::ByteVector | MemoryType::ByteSliceMut => true,
        MemoryType::List(inner) => verified_affine_or_resource(inner),
        MemoryType::Enum { arguments, .. } => arguments.iter().any(verified_affine_or_resource),
        MemoryType::Function { parameters, result } => {
            parameters.iter().any(verified_affine_or_resource)
                || verified_affine_or_resource(result)
        }
        MemoryType::ForAll { body, .. } => verified_affine_or_resource(body),
        _ => false,
    }
}

fn verified_route(
    fact: &ExprFact<'_>,
    last_use: bool,
    independent: u64,
    nodes: u64,
    bytes: u64,
    sealed: bool,
) -> MemoryValueRoute {
    if matches!(fact.expression.kind,
        hir::ExprKind::Borrow { .. } | hir::ExprKind::BorrowBytes { .. })
    {
        MemoryValueRoute::Borrow
    } else if last_use || matches!(fact.expression.kind, hir::ExprKind::Move { .. }) {
        MemoryValueRoute::LastUseMove
    } else if matches!(fact.expression.kind, hir::ExprKind::WithProductField { .. }) {
        MemoryValueRoute::UniqueReuse
    } else if independent >= 2 && nodes < INITIAL_SEAL_NODES && bytes < INITIAL_SEAL_BYTES {
        MemoryValueRoute::DetachedClone
    } else if sealed {
        MemoryValueRoute::SealedShare
    } else if matches!(fact.expression.kind, hir::ExprKind::Load(_)) {
        MemoryValueRoute::DetachedClone
    } else {
        MemoryValueRoute::UniqueReuse
    }
}

fn verified_storage(route: MemoryValueRoute, sealed: bool) -> MemoryDomain {
    if route == MemoryValueRoute::Borrow {
        MemoryDomain::BorrowedView
    } else if sealed {
        MemoryDomain::SealedRegion
    } else {
        MemoryDomain::UniqueStructural
    }
}

fn verified_cleanup(
    route: MemoryValueRoute,
    storage: MemoryDomain,
) -> MemoryValueFailureCleanup {
    match (route, storage) {
        (MemoryValueRoute::Borrow, _) => MemoryValueFailureCleanup::EndBorrow,
        (_, MemoryDomain::SealedRegion) => MemoryValueFailureCleanup::DisposeSealedOwner,
        (_, MemoryDomain::UniqueStructural) => MemoryValueFailureCleanup::DisposeUniqueOwner,
        _ => MemoryValueFailureCleanup::None,
    }
}

fn verified_representation_id(
    type_fact: &MemoryTypeFact,
    category: MemoryValueCategory,
    storage: MemoryDomain,
    route: MemoryValueRoute,
) -> Result<MemoryValueRepresentationId> {
    let mut bytes = b"lkjscript.memory-value-representation\0canonical-platform-contract".to_vec();
    bytes.extend_from_slice(&type_fact.witness.as_bytes());
    let _ = route;
    bytes.extend_from_slice(&[
        verified_category_tag(category),
        verified_storage_tag(storage),
    ]);
    Ok(MemoryValueRepresentationId::from_bytes(lkjscript_core::sha256(&bytes)))
}

fn verified_category_tag(value: MemoryValueCategory) -> u8 {
    match value {
        MemoryValueCategory::Owner => 0,
        MemoryValueCategory::View => 1,
        MemoryValueCategory::Destination => 2,
    }
}
fn verified_storage_tag(value: MemoryDomain) -> u8 {
    match value {
        MemoryDomain::Inline => 0, MemoryDomain::Static => 1, MemoryDomain::Stack => 2,
        MemoryDomain::CallerDestination => 3, MemoryDomain::UniqueStructural => 4,
        MemoryDomain::OrdinaryRegion => 5, MemoryDomain::SealedRegion => 6,
        MemoryDomain::BorrowedView => 7, MemoryDomain::ExternalResource => 8,
        MemoryDomain::UnsupportedRuntime => 9,
    }
}
