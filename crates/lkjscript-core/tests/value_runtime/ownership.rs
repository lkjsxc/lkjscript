use lkjscript_core::{
    SemanticPayload, SemanticValue, StructuralEventKind, StructuralFieldPath, StructuralKind,
    StructuralProjection, StructuralRootTableError, StructuralValueError,
};

use super::support::{publish, runtime, value_type};

#[test]
fn move_invalidates_the_old_key_and_drop_releases_exactly_once() -> Result<(), StructuralValueError>
{
    let string_type = value_type(41, 42, StructuralKind::String)?;
    let mut runtime = runtime()?;
    let key = publish(
        &mut runtime,
        SemanticValue::new(string_type, SemanticPayload::String(b"move".to_vec())),
    )?;
    let moved = runtime.move_owned(key, string_type)?;
    assert_eq!(moved.slot(), key.slot());
    assert_ne!(moved.generation(), key.generation());
    assert_eq!(
        runtime.value(key, string_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::StaleRoot
        ))
    );

    runtime.drop_owned(moved, string_type)?;
    let metrics = runtime.metrics();
    assert_eq!(metrics.allocations, 1);
    assert_eq!(metrics.moves, 1);
    assert_eq!(metrics.drops, 1);
    assert_eq!(metrics.releases, 1);
    assert_eq!(metrics.release_work, 1);
    assert_eq!(metrics.string_bytes_released, 4);
    assert_eq!(metrics.live_objects, 0);
    runtime.verify_empty()
}

#[test]
fn explicit_clone_is_deep_and_semantic_export_contains_no_runtime_key(
) -> Result<(), StructuralValueError> {
    let vector_type = value_type(43, 44, StructuralKind::ByteVector)?;
    let mut runtime = runtime()?;
    let original_value =
        SemanticValue::new(vector_type, SemanticPayload::ByteVector(vec![1, 2, 3, 4]));
    let original = publish(&mut runtime, original_value.clone())?;
    let copy = runtime.clone_owned(original, vector_type)?;
    let view = runtime.borrow_projected(
        copy,
        vector_type,
        StructuralProjection::Field {
            path: StructuralFieldPath::root(),
            expected: vector_type,
        },
        true,
    )?;
    runtime.byte_vector_mut(view)?[0] = 9;
    runtime.end_view(view)?;
    assert_eq!(runtime.value(original, vector_type)?, original_value);

    let exported = runtime.export_semantic(copy, vector_type)?;
    assert_eq!(
        exported.payload,
        SemanticPayload::ByteVector(vec![9, 2, 3, 4])
    );
    assert!(!format!("{exported:?}").contains("StructuralValueKey"));
    assert_eq!(
        runtime.value(copy, vector_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::MovedRoot
        ))
    );
    assert_eq!(runtime.metrics().clones, 1);
    assert_eq!(runtime.metrics().clone_nodes, 1);
    runtime.drop_owned(original, vector_type)?;
    runtime.verify_empty()
}

#[test]
fn event_log_and_leak_checks_report_exact_lifecycle() -> Result<(), StructuralValueError> {
    let string_type = value_type(45, 46, StructuralKind::String)?;
    let mut runtime = lkjscript_core::StructuralValueRuntime::new()?;
    let key = publish(
        &mut runtime,
        SemanticValue::new(string_type, SemanticPayload::String(b"event".to_vec())),
    )?;
    assert_eq!(
        runtime.verify_empty(),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::LiveRoot
        ))
    );
    let moved = runtime.move_owned(key, string_type)?;
    runtime.drop_owned(moved, string_type)?;

    let events: Vec<_> = runtime.events().iter().copied().collect();
    assert_eq!(events.len(), 5);
    assert_eq!(events[3].kind, StructuralEventKind::Drop);
    assert_eq!(events[3].sequence, 4);
    assert_eq!(events[4].kind, StructuralEventKind::Release);
    assert_eq!(events[4].sequence, 5);
    assert_eq!(runtime.metrics().events_overwritten, 0);
    runtime.verify_empty()
}
