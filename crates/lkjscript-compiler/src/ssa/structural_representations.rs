use crate::memory_plan::{MemoryDomain, MemoryValueCategory};
use crate::ssa::*;

type RepresentationKey = (
    StructuralTypeId,
    MemoryWitnessId,
    MemoryWitnessGroupId,
    u64,
    StructuralLayoutId,
    StructuralValueCategory,
    StructuralStorage,
    [u8; 32],
);

pub(super) fn install_value_representations(
    memory: &mut StructuralMemoryMetadata,
    plan: &HirMemoryPlan,
    products: &HashMap<crate::hir::ProductId, ProductId>,
) -> Result<()> {
    let mut representation_keys = HashSet::new();
    let mut facts_by_witness = HashMap::new();
    for fact in &plan.type_facts {
        if facts_by_witness.insert(fact.witness, fact).is_some() {
            return Err(Error::msg("structural type fact witness is duplicated"));
        }
    }
    let mut witnesses_by_id = HashMap::new();
    for witness in &plan.witnesses {
        if witnesses_by_id.insert(witness.id, witness).is_some() {
            return Err(Error::msg("structural witness identity is duplicated"));
        }
    }
    let mut placements_by_type = HashMap::new();
    for placement in &plan.value_placements {
        let placements: &mut Vec<_> = placements_by_type.entry(placement.type_fact).or_default();
        placements
            .try_reserve(1)
            .map_err(|_| Error::host("structural placement index allocation failed"))?;
        placements.push(placement);
    }
    let types = memory.types.clone();
    for structural in types {
        let fact = facts_by_witness
            .get(&crate::memory_plan::MemoryWitnessId::from_bytes(
                structural.witness.bytes(),
            ))
            .copied()
            .filter(|fact| {
                lower_memory_type(&fact.ty, products).ok().as_ref() == Some(&structural.ty)
            })
            .ok_or_else(|| Error::msg("structural representation lost exact type fact"))?;
        let witness = witnesses_by_id
            .get(&fact.witness)
            .copied()
            .ok_or_else(|| Error::msg("structural representation lost witness member"))?;
        let unique = fallback_route(
            fact.witness,
            MemoryValueCategory::Owner,
            MemoryDomain::UniqueStructural,
            2,
        )?;
        push_representation(
            memory,
            &mut representation_keys,
            &structural,
            witness,
            StructuralValueCategory::Owner,
            StructuralStorage::UniqueStructural,
            unique,
        )?;
        push_representation(
            memory,
            &mut representation_keys,
            &structural,
            witness,
            StructuralValueCategory::Destination,
            StructuralStorage::UniqueStructural,
            unique,
        )?;
        let borrowed = fallback_route(
            fact.witness,
            MemoryValueCategory::View,
            MemoryDomain::BorrowedView,
            0,
        )?;
        push_representation(
            memory,
            &mut representation_keys,
            &structural,
            witness,
            StructuralValueCategory::View,
            StructuralStorage::BorrowedView,
            borrowed,
        )?;
        for placement in placements_by_type.get(&fact.id).into_iter().flatten() {
            let storage = lower_storage(placement.storage)?;
            let category = lower_category(placement.category);
            push_representation(
                memory,
                &mut representation_keys,
                &structural,
                witness,
                category,
                storage,
                placement.representation.as_bytes(),
            )?;
            if placement.category == MemoryValueCategory::Owner {
                push_representation(
                    memory,
                    &mut representation_keys,
                    &structural,
                    witness,
                    StructuralValueCategory::Destination,
                    storage,
                    placement.representation.as_bytes(),
                )?;
            }
        }
    }
    memory.representations.sort_by_key(|item| {
        (
            item.type_id,
            item.category,
            item.storage,
            item.route,
            item.witness,
            item.witness_group,
            item.witness_member,
            item.layout,
        )
    });
    for (index, item) in memory.representations.iter_mut().enumerate() {
        item.id = StructuralRepresentationId::new(
            u64::try_from(index)
                .map_err(|_| Error::msg("structural representation table exceeds u64"))?,
        );
    }
    Ok(())
}

fn push_representation(
    memory: &mut StructuralMemoryMetadata,
    keys: &mut HashSet<RepresentationKey>,
    ty: &StructuralTypeMetadata,
    witness: &crate::memory_plan::MemoryWitness,
    category: StructuralValueCategory,
    storage: StructuralStorage,
    route: [u8; 32],
) -> Result<()> {
    let group = MemoryWitnessGroupId::new(
        witness
            .group
            .ok_or_else(|| Error::msg("structural witness has no group"))?
            .as_bytes(),
    );
    let ordinal = witness
        .ordinal
        .ok_or_else(|| Error::msg("structural witness has no ordinal"))?;
    if !keys.insert((
        ty.id, ty.witness, group, ordinal, ty.layout, category, storage, route,
    )) {
        return Ok(());
    }
    let id = StructuralRepresentationId::new(
        u64::try_from(memory.representations.len())
            .map_err(|_| Error::msg("structural representation table exceeds u64"))?,
    );
    memory
        .representations
        .push(StructuralRepresentationMetadata {
            id,
            type_id: ty.id,
            witness: ty.witness,
            witness_group: group,
            witness_member: ordinal,
            layout: ty.layout,
            category,
            storage,
            route,
        });
    Ok(())
}

fn lower_category(value: MemoryValueCategory) -> StructuralValueCategory {
    match value {
        MemoryValueCategory::Owner => StructuralValueCategory::Owner,
        MemoryValueCategory::View => StructuralValueCategory::View,
        MemoryValueCategory::Destination => StructuralValueCategory::Destination,
    }
}

fn lower_storage(value: MemoryDomain) -> Result<StructuralStorage> {
    Ok(match value {
        MemoryDomain::Inline => StructuralStorage::Inline,
        MemoryDomain::Static => StructuralStorage::Static,
        MemoryDomain::Stack => StructuralStorage::Stack,
        MemoryDomain::CallerDestination => StructuralStorage::CallerDestination,
        MemoryDomain::UniqueStructural => StructuralStorage::UniqueStructural,
        MemoryDomain::OrdinaryRegion => StructuralStorage::OrdinaryRegion,
        MemoryDomain::SealedRegion => StructuralStorage::SealedRegion,
        MemoryDomain::BorrowedView => StructuralStorage::BorrowedView,
        MemoryDomain::ExternalResource => StructuralStorage::ExternalResource,
        MemoryDomain::UnsupportedRuntime => {
            return Err(Error::msg("unsupported placement reached SSA"))
        }
    })
}

pub(super) fn fallback_route(
    witness: crate::memory_plan::MemoryWitnessId,
    category: MemoryValueCategory,
    storage: MemoryDomain,
    route: u8,
) -> Result<[u8; 32]> {
    let mut bytes = b"lkjscript.memory-value-representation\0canonical-platform-contract".to_vec();
    bytes.extend_from_slice(&witness.as_bytes());
    let _ = route;
    bytes.extend_from_slice(&[category_tag(category), storage_tag(storage)]);
    Ok(lkjscript_core::sha256(&bytes))
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
        MemoryDomain::Inline => 0,
        MemoryDomain::Static => 1,
        MemoryDomain::Stack => 2,
        MemoryDomain::CallerDestination => 3,
        MemoryDomain::UniqueStructural => 4,
        MemoryDomain::OrdinaryRegion => 5,
        MemoryDomain::SealedRegion => 6,
        MemoryDomain::BorrowedView => 7,
        MemoryDomain::ExternalResource => 8,
        MemoryDomain::UnsupportedRuntime => 9,
    }
}
