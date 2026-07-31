use lkjscript_core::{
    SemanticPayload, SemanticValue, StructuralError, StructuralFieldPath, StructuralKind,
    StructuralLimit, StructuralProjection, StructuralRootTableError, StructuralRootTableLimit,
    StructuralValueError, StructuralValueLimit, StructuralValueRuntime,
    StructuralValueRuntimeLimits,
};

use super::support::{publish, publish_failure, value_type};

#[test]
fn root_capacity_failure_rolls_back_object_and_domain_staging() -> Result<(), StructuralValueError>
{
    let string_type = value_type(61, 62, StructuralKind::String)?;
    let mut limits = StructuralValueRuntimeLimits::default();
    limits.roots.max_roots = 1;
    limits.max_objects = 2;
    limits.domains.max_domains = 2;
    let mut runtime = StructuralValueRuntime::new(limits)?;
    let first = publish(
        &mut runtime,
        SemanticValue::new(string_type, SemanticPayload::String(b"first".to_vec())),
    )?;
    let rejected = SemanticValue::new(string_type, SemanticPayload::String(b"rejected".to_vec()));
    let failure = publish_failure(runtime.publish_owned(rejected.clone()))?;
    assert_eq!(
        failure.error,
        StructuralValueError::RootTable(StructuralRootTableError::LimitExceeded(
            StructuralRootTableLimit::Roots
        ))
    );
    assert_eq!(failure.value, rejected);
    assert_eq!(runtime.metrics().live_objects, 1);
    assert_eq!(runtime.root_stats().live_roots, 1);

    runtime.drop_owned(first, string_type)?;
    let replacement = publish(&mut runtime, failure.value)?;
    assert_eq!(replacement.slot(), first.slot());
    assert_ne!(replacement.generation(), first.generation());
    assert_eq!(runtime.metrics().object_slots_reused, 1);
    runtime.drop_owned(replacement, string_type)?;
    runtime.verify_empty()
}

#[test]
fn object_stage_failure_restores_the_input_and_domain_capacity() -> Result<(), StructuralValueError>
{
    let string_type = value_type(63, 64, StructuralKind::String)?;
    let mut limits = StructuralValueRuntimeLimits::default();
    limits.domains.max_domains = 2;
    limits.roots.max_roots = 2;
    let limits = StructuralValueRuntimeLimits {
        max_objects: 1,
        ..limits
    };
    let mut runtime = StructuralValueRuntime::new(limits)?;
    let first = publish(
        &mut runtime,
        SemanticValue::new(string_type, SemanticPayload::String(b"first".to_vec())),
    )?;
    let rejected = SemanticValue::new(string_type, SemanticPayload::String(b"second".to_vec()));
    let failure = publish_failure(runtime.publish_owned(rejected.clone()))?;
    assert_eq!(
        failure.error,
        StructuralValueError::LimitExceeded(StructuralValueLimit::Objects)
    );
    assert_eq!(failure.value, rejected);
    assert_eq!(runtime.metrics().live_objects, 1);
    assert_eq!(runtime.root_stats().live_roots, 1);
    runtime.drop_owned(first, string_type)?;
    let replacement = publish(&mut runtime, failure.value)?;
    runtime.drop_owned(replacement, string_type)?;
    runtime.verify_empty()
}

#[test]
fn generation_retirement_becomes_capacity_without_live_partial_state(
) -> Result<(), StructuralValueError> {
    let string_type = value_type(63, 64, StructuralKind::String)?;
    let mut limits = StructuralValueRuntimeLimits::default();
    limits.domains.max_domains = 1;
    limits.domains.max_generation = 2;
    limits.roots.max_roots = 1;
    limits.roots.max_generation = 2;
    let limits = StructuralValueRuntimeLimits {
        max_objects: 1,
        max_generation: 2,
        ..limits
    };
    let mut runtime = StructuralValueRuntime::new(limits)?;
    let first = publish(
        &mut runtime,
        SemanticValue::new(string_type, SemanticPayload::String(b"one".to_vec())),
    )?;
    runtime.drop_owned(first, string_type)?;
    let second = publish(
        &mut runtime,
        SemanticValue::new(string_type, SemanticPayload::String(b"two".to_vec())),
    )?;
    assert_eq!(second.slot(), first.slot());
    assert_eq!(second.generation(), 2);
    runtime.drop_owned(second, string_type)?;
    let third = SemanticValue::new(string_type, SemanticPayload::String(b"three".to_vec()));
    let failure = publish_failure(runtime.publish_owned(third.clone()))?;
    assert_eq!(
        failure.error,
        StructuralValueError::Domain(StructuralError::LimitExceeded(StructuralLimit::Domains))
    );
    assert_eq!(failure.value, third);
    assert_eq!(runtime.root_stats().root_slots_retired, 1);
    assert_eq!(runtime.metrics().live_objects, 0);
    runtime.verify_empty()
}

#[test]
fn view_and_destination_capacity_fail_without_partial_mutation() -> Result<(), StructuralValueError>
{
    let string_type = value_type(65, 66, StructuralKind::String)?;
    let product_type = value_type(67, 68, StructuralKind::Product)?;
    let limits = StructuralValueRuntimeLimits {
        max_views: 1,
        max_destinations: 1,
        ..StructuralValueRuntimeLimits::default()
    };
    let mut runtime = StructuralValueRuntime::new(limits)?;
    let key = publish(
        &mut runtime,
        SemanticValue::new(string_type, SemanticPayload::String(b"view".to_vec())),
    )?;
    let projection = StructuralProjection::Field {
        path: StructuralFieldPath::root(),
        expected: string_type,
    };
    let first = runtime.borrow_projected(key, string_type, projection.clone(), false)?;
    assert_eq!(
        runtime.borrow_projected(key, string_type, projection, false),
        Err(StructuralValueError::LimitExceeded(
            StructuralValueLimit::Views
        ))
    );
    assert_eq!(runtime.root_stats().live_loans, 1);
    assert_eq!(runtime.metrics().live_views, 1);
    runtime.end_view(first)?;
    runtime.drop_owned(key, string_type)?;

    let destination = runtime.begin_product(product_type, Vec::new())?;
    assert_eq!(
        runtime.begin_product(product_type, Vec::new()),
        Err(StructuralValueError::LimitExceeded(
            StructuralValueLimit::Destinations
        ))
    );
    assert_eq!(runtime.metrics().live_destinations, 1);
    assert_eq!(
        runtime.verify_empty(),
        Err(StructuralValueError::LiveDestination)
    );
    runtime.abort_destination(destination)?;
    runtime.verify_empty()
}
