use crate::memory_plan::{MemoryDomain, MemoryValueCategory};
use crate::ssa::*;

pub(super) fn install_value_representations(
    memory: &mut StructuralMemoryMetadata,
    plan: &HirMemoryPlan,
    products: &HashMap<String, ProductId>,
) -> Result<()> {
    let types = memory.types.clone();
    for structural in types {
        let fact = plan
            .type_facts
            .iter()
            .find(|fact| {
                fact.witness.as_bytes() == structural.witness.bytes()
                    && lower_memory_type(&fact.ty, products).ok().as_ref() == Some(&structural.ty)
            })
            .ok_or_else(|| Error::msg("structural representation lost exact type fact"))?;
        let witness = plan
            .witness(fact.witness)
            .ok_or_else(|| Error::msg("structural representation lost witness member"))?;
        let unique = fallback_route(
            fact.witness,
            MemoryValueCategory::Owner,
            MemoryDomain::UniqueStructural,
            2,
        )?;
        push_representation(
            memory,
            &structural,
            witness,
            StructuralValueCategory::Owner,
            StructuralStorage::UniqueStructural,
            unique,
        )?;
        push_representation(
            memory,
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
            &structural,
            witness,
            StructuralValueCategory::View,
            StructuralStorage::BorrowedView,
            borrowed,
        )?;
        for placement in plan
            .value_placements
            .iter()
            .filter(|item| item.type_fact == fact.id)
        {
            let storage = lower_storage(placement.storage)?;
            let category = lower_category(placement.category);
            push_representation(
                memory,
                &structural,
                witness,
                category,
                storage,
                placement.representation.as_bytes(),
            )?;
            if placement.category == MemoryValueCategory::Owner {
                push_representation(
                    memory,
                    &structural,
                    witness,
                    StructuralValueCategory::Destination,
                    storage,
                    placement.representation.as_bytes(),
                )?;
            }
        }
    }
    Ok(())
}

fn push_representation(
    memory: &mut StructuralMemoryMetadata,
    ty: &StructuralTypeMetadata,
    witness: &crate::memory_plan::MemoryWitness,
    category: StructuralValueCategory,
    storage: StructuralStorage,
    route: [u8; 32],
) -> Result<()> {
    let duplicate = memory.representations.iter().any(|item| {
        item.type_id == ty.id
            && item.witness == ty.witness
            && item.witness_group.bytes() == witness.group.as_bytes()
            && item.witness_member == witness.ordinal
            && item.layout == ty.layout
            && item.category == category
            && item.storage == storage
            && item.route == route
    });
    if duplicate {
        return Ok(());
    }
    let id = StructuralRepresentationId::new(
        u16::try_from(memory.representations.len())
            .map_err(|_| Error::msg("structural representation table exceeds u16"))?,
    );
    memory
        .representations
        .push(StructuralRepresentationMetadata {
            id,
            type_id: ty.id,
            witness: ty.witness,
            witness_group: lkjscript_ir::MemoryWitnessGroupId::new(witness.group.as_bytes()),
            witness_member: witness.ordinal,
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
