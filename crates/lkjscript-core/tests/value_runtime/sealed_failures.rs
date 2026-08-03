use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralKind,
    StructuralRootTableError, StructuralRootTableLimit, StructuralValueError,
    StructuralValueRuntime, StructuralValueRuntimeLimits,
};

use super::support::{publish, runtime, value_type};

#[test]
fn copied_key_double_release_stale_type_runtime_and_overflow_reject(
) -> Result<(), StructuralValueError> {
    let sealed_type = value_type(121, 122, StructuralKind::String)?;
    let wrong_layout = value_type(123, 122, StructuralKind::String)?;
    let mut limits = StructuralValueRuntimeLimits::default();
    limits.domains.max_region_owners = 1;
    let mut runtime = StructuralValueRuntime::new(limits)?;
    let unique = publish(
        &mut runtime,
        SemanticValue::new(sealed_type, SemanticPayload::String(b"reject".to_vec())),
    )?;
    let owner = runtime.seal_owned(unique, sealed_type)?.owner;
    let copied_bits = owner;
    assert_eq!(
        runtime.acquire_sealed(owner, sealed_type),
        Err(StructuralValueError::OwnerOverflow)
    );
    assert_eq!(runtime.sealed_owners_for(owner, sealed_type)?, 1);
    assert_eq!(
        runtime.acquire_sealed(owner, wrong_layout),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::WrongLayout
        ))
    );
    let mut foreign = super::support::runtime()?;
    assert_eq!(
        foreign.acquire_sealed(owner, sealed_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::StaleRoot
        ))
    );
    runtime.dispose_owner(owner, sealed_type)?;
    assert_eq!(
        runtime.dispose_owner(copied_bits, sealed_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::DroppedRoot
        ))
    );
    assert_eq!(runtime.metrics().live_sealed_owners, 0);
    runtime.verify_empty()
}

#[test]
fn sealing_and_acquisition_preflight_preserve_existing_ownership(
) -> Result<(), StructuralValueError> {
    let value_type = value_type(123, 124, StructuralKind::String)?;
    let mut generation_limits = StructuralValueRuntimeLimits::default();
    generation_limits.roots.max_generation = 1;
    let mut generation_runtime = StructuralValueRuntime::new(generation_limits)?;
    let unique = publish(
        &mut generation_runtime,
        SemanticValue::new(value_type, SemanticPayload::String(b"unique".to_vec())),
    )?;
    assert_eq!(
        generation_runtime.seal_owned(unique, value_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::GenerationExhausted
        ))
    );
    assert_eq!(
        generation_runtime.value(unique, value_type)?.payload,
        SemanticPayload::String(b"unique".to_vec())
    );
    generation_runtime.dispose_owner(unique, value_type)?;
    generation_runtime.verify_empty()?;

    let mut root_limits = StructuralValueRuntimeLimits::default();
    root_limits.roots.max_roots = 1;
    let mut root_runtime = StructuralValueRuntime::new(root_limits)?;
    let unique = publish(
        &mut root_runtime,
        SemanticValue::new(value_type, SemanticPayload::String(b"sealed".to_vec())),
    )?;
    let owner = root_runtime.seal_owned(unique, value_type)?.owner;
    assert_eq!(
        root_runtime.acquire_sealed(owner, value_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::LimitExceeded(StructuralRootTableLimit::Roots)
        ))
    );
    assert_eq!(root_runtime.sealed_owners_for(owner, value_type)?, 1);
    root_runtime.dispose_owner(owner, value_type)?;
    root_runtime.verify_empty()
}

#[test]
fn direct_destination_finishes_as_sealed_without_unique_publication(
) -> Result<(), StructuralValueError> {
    let product = value_type(125, 126, StructuralKind::Product)?;
    let scalar = value_type(127, 128, StructuralKind::I64)?;
    let mut runtime = runtime()?;
    let destination = runtime.begin_product(product, vec![scalar])?;
    runtime
        .initialize_node(
            destination,
            0,
            SemanticValue::new(
                scalar,
                SemanticPayload::Inline(InlineStructuralValue::I64(7)),
            ),
        )
        .map_err(|failure| failure.error)?;
    let sealed = runtime.finish_destination_sealed(destination)?;
    assert!(!sealed.zero_copy_adopted);
    assert_eq!(runtime.metrics().sealed_publications, 1);
    assert!(runtime.metrics().copied_publication_bytes > 0);
    assert_eq!(runtime.root_stats().live_roots, 1);
    assert_eq!(runtime.metrics().live_destinations, 0);
    runtime.dispose_owner(sealed.owner, product)?;
    runtime.verify_empty()
}

#[test]
fn failed_sealed_publication_rolls_back_domain_object_and_owner() -> Result<(), StructuralValueError>
{
    let product = value_type(129, 130, StructuralKind::Product)?;
    let scalar = value_type(131, 132, StructuralKind::I64)?;
    let mut limits = StructuralValueRuntimeLimits::default();
    limits.roots.max_roots = 1;
    limits.max_objects = 2;
    limits.domains.max_domains = 2;
    let mut runtime = StructuralValueRuntime::new(limits)?;
    let occupied = publish(
        &mut runtime,
        SemanticValue::new(
            scalar,
            SemanticPayload::Inline(InlineStructuralValue::I64(1)),
        ),
    )?;
    let destination = runtime.begin_product(product, vec![scalar])?;
    runtime
        .initialize_node(
            destination,
            0,
            SemanticValue::new(
                scalar,
                SemanticPayload::Inline(InlineStructuralValue::I64(2)),
            ),
        )
        .map_err(|failure| failure.error)?;
    assert_eq!(
        runtime.finish_destination_sealed(destination),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::LimitExceeded(StructuralRootTableLimit::Roots)
        ))
    );
    assert_eq!(runtime.root_stats().live_roots, 1);
    assert_eq!(runtime.metrics().live_objects, 1);
    assert_eq!(runtime.metrics().live_sealed_domains, 0);
    assert_eq!(runtime.metrics().live_sealed_owners, 0);
    assert_eq!(runtime.metrics().sealed_publications, 0);
    runtime.abort_destination(destination)?;
    runtime.dispose_owner(occupied, scalar)?;
    runtime.verify_empty()
}

#[test]
fn incomplete_direct_destination_publishes_nothing_and_teardown_is_exact(
) -> Result<(), StructuralValueError> {
    let product = value_type(133, 134, StructuralKind::Product)?;
    let scalar = value_type(135, 136, StructuralKind::I64)?;
    let mut runtime = runtime()?;
    let destination = runtime.begin_product(product, vec![scalar])?;
    assert_eq!(
        runtime.finish_destination_sealed(destination),
        Err(StructuralValueError::IncompleteDestination)
    );
    assert_eq!(runtime.root_stats().live_roots, 0);
    assert_eq!(runtime.metrics().live_objects, 0);
    assert_eq!(runtime.metrics().live_sealed_domains, 0);
    runtime.abort_destination(destination)?;
    let metrics = runtime.metrics();
    assert_eq!(metrics.live_views, 0);
    assert_eq!(metrics.live_destinations, 0);
    assert_eq!(metrics.live_sealed_owners, 0);
    assert_eq!(metrics.release_backlog, 0);
    runtime.verify_empty()
}
