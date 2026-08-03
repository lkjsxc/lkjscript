use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralFieldPath, StructuralKind,
    StructuralOwnerKind, StructuralProjection, StructuralRootTableError, StructuralValueError,
};

use super::support::{publish, runtime, value_type};

#[test]
fn unique_image_seals_in_place_and_invalidates_unique_key() -> Result<(), StructuralValueError> {
    let kind = StructuralKind::String;
    let value_type = value_type(101, 102, kind)?;
    let mut runtime = runtime()?;
    let unique = publish(
        &mut runtime,
        SemanticValue::new(value_type, SemanticPayload::String(b"sealed".to_vec())),
    )?;
    let sealed = runtime.seal_owned(unique, value_type)?;
    assert!(sealed.zero_copy_adopted);
    assert_eq!(sealed.owner.slot(), unique.slot());
    assert_ne!(sealed.owner.generation(), unique.generation());
    assert_eq!(
        runtime.value(unique, value_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::StaleRoot
        ))
    );
    assert_eq!(
        runtime.value(sealed.owner, value_type)?.payload,
        SemanticPayload::String(b"sealed".to_vec())
    );
    let metrics = runtime.metrics();
    assert_eq!(metrics.zero_copy_adoptions, 1);
    assert_eq!(metrics.copied_publication_bytes, 0);
    assert_eq!(metrics.live_objects, 1);
    assert_eq!(metrics.live_sealed_domains, 1);
    let report = runtime.dispose_owner(sealed.owner, value_type)?;
    assert_eq!(report.ownership, StructuralOwnerKind::Sealed);
    assert!(report.final_release);
    assert_eq!(report.nodes_reclaimed, 1);
    runtime.verify_empty()
}

#[test]
fn independent_owners_share_one_image_and_move_is_count_free() -> Result<(), StructuralValueError> {
    let value_type = value_type(103, 104, StructuralKind::String)?;
    let mut runtime = runtime()?;
    let unique = publish(
        &mut runtime,
        SemanticValue::new(value_type, SemanticPayload::String(b"owners".to_vec())),
    )?;
    let first = runtime.seal_owned(unique, value_type)?.owner;
    let second = runtime.acquire_sealed(first, value_type)?;
    let third = runtime.acquire_sealed(first, value_type)?;
    let fourth = runtime.acquire_sealed(first, value_type)?;
    assert_eq!(runtime.sealed_owners_for(first, value_type)?, 4);
    assert_eq!(runtime.metrics().live_objects, 1);
    assert_eq!(runtime.metrics().live_sealed_domains, 1);
    let moved = runtime.move_sealed(fourth, value_type)?;
    assert_eq!(runtime.sealed_owners_for(moved, value_type)?, 4);
    assert_eq!(runtime.metrics().sealed_acquisitions, 3);
    for owner in [first, second, third] {
        let report = runtime.dispose_owner(owner, value_type)?;
        assert!(!report.final_release);
        assert_eq!(report.nodes_reclaimed, 0);
    }
    let report = runtime.dispose_owner(moved, value_type)?;
    assert!(report.final_release);
    assert_eq!(runtime.metrics().sealed_releases, 4);
    runtime.verify_empty()
}

#[test]
fn sealed_borrow_has_no_owner_traffic_and_blocks_owner_release() -> Result<(), StructuralValueError>
{
    let value_type = value_type(105, 106, StructuralKind::String)?;
    let mut runtime = runtime()?;
    let unique = publish(
        &mut runtime,
        SemanticValue::new(value_type, SemanticPayload::String(b"borrow".to_vec())),
    )?;
    let owner = runtime.seal_owned(unique, value_type)?.owner;
    let before = runtime.metrics();
    let view = runtime.borrow_projected(
        owner,
        value_type,
        StructuralProjection::Field {
            path: StructuralFieldPath::root(),
            expected: value_type,
        },
        false,
    )?;
    assert_eq!(
        runtime.metrics().sealed_acquisitions,
        before.sealed_acquisitions
    );
    assert_eq!(runtime.metrics().sealed_releases, before.sealed_releases);
    assert_eq!(
        runtime.dispose_owner(owner, value_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::LiveLoan
        ))
    );
    assert_eq!(runtime.sealed_owners_for(owner, value_type)?, 1);
    runtime.end_view(view)?;
    runtime.dispose_owner(owner, value_type)?;
    runtime.verify_empty()
}

#[test]
fn sealed_release_planning_is_independent_of_image_nodes() -> Result<(), StructuralValueError> {
    let small = release_case(8, 111, 112)?;
    let large = release_case(2_048, 113, 114)?;
    assert_eq!(small.0, large.0);
    assert_eq!(small.0, 5);
    assert_eq!(small.1, 8);
    assert_eq!(large.1, 2_048);
    Ok(())
}

fn release_case(
    nodes: usize,
    layout: u64,
    semantic: u64,
) -> Result<(u64, u32), StructuralValueError> {
    let product = value_type(layout, semantic, StructuralKind::Product)?;
    let scalar = value_type(layout + 100, semantic + 100, StructuralKind::I64)?;
    let limits = lkjscript_core::StructuralValueRuntimeLimits {
        max_fields: 4_096,
        ..lkjscript_core::StructuralValueRuntimeLimits::default()
    };
    let mut runtime = lkjscript_core::StructuralValueRuntime::new(limits)?;
    let children = (1..nodes)
        .map(|index| {
            SemanticValue::new(
                scalar,
                SemanticPayload::Inline(InlineStructuralValue::I64(index as i64)),
            )
        })
        .collect::<Vec<_>>();
    let unique = publish(
        &mut runtime,
        SemanticValue::new(product, SemanticPayload::Product(children.into())),
    )?;
    let first = runtime.seal_owned(unique, product)?.owner;
    let owners = [
        first,
        runtime.acquire_sealed(first, product)?,
        runtime.acquire_sealed(first, product)?,
        runtime.acquire_sealed(first, product)?,
        runtime.acquire_sealed(first, product)?,
    ];
    let before = runtime.metrics().sealed_release_work;
    let mut reclaimed = 0;
    for owner in owners {
        reclaimed += runtime.dispose_owner(owner, product)?.nodes_reclaimed;
    }
    let work = runtime.metrics().sealed_release_work - before;
    assert_eq!(
        runtime.metrics().sealed_nodes_reclaimed,
        u64::from(reclaimed)
    );
    runtime.verify_empty()?;
    Ok((work, reclaimed))
}
