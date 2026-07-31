use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralKind, StructuralValueError,
    StructuralValueLimit, StructuralValueRuntime, StructuralValueRuntimeLimits,
};

use super::support::{runtime, value_type};

#[test]
fn product_destination_handles_double_init_incomplete_finish_and_success(
) -> Result<(), StructuralValueError> {
    let product_type = value_type(71, 72, StructuralKind::Product)?;
    let i64_type = value_type(73, 74, StructuralKind::I64)?;
    let string_type = value_type(75, 76, StructuralKind::String)?;
    let mut runtime = runtime()?;
    let destination = runtime.begin_product(product_type, vec![i64_type, string_type])?;
    let string = SemanticValue::new(
        string_type,
        SemanticPayload::String(b"destination".to_vec()),
    );
    runtime
        .initialize_node(destination, 1, string.clone())
        .map_err(|failure| failure.error)?;
    let duplicate = SemanticValue::new(string_type, SemanticPayload::String(b"duplicate".to_vec()));
    let failure = match runtime.initialize_node(destination, 1, duplicate.clone()) {
        Ok(()) => return Err(StructuralValueError::InvariantViolation),
        Err(failure) => failure,
    };
    assert_eq!(failure.error, StructuralValueError::FieldAlreadyInitialized);
    assert_eq!(failure.value, duplicate);
    assert_eq!(
        runtime.finish_destination(destination),
        Err(StructuralValueError::IncompleteDestination)
    );
    let integer = SemanticValue::new(
        i64_type,
        SemanticPayload::Inline(InlineStructuralValue::I64(99)),
    );
    runtime
        .initialize_node(destination, 0, integer.clone())
        .map_err(|failure| failure.error)?;
    let key = runtime.finish_destination(destination)?;
    assert_eq!(
        runtime.finish_destination(destination),
        Err(StructuralValueError::StaleDestination)
    );
    assert_eq!(
        runtime.value(key, product_type)?.payload,
        SemanticPayload::Product(vec![integer, string].into())
    );
    assert_eq!(runtime.metrics().initializations, 2);
    assert_eq!(runtime.metrics().destinations_completed, 1);
    assert_eq!(runtime.metrics().live_destinations, 0);
    runtime.drop_owned(key, product_type)?;
    runtime.verify_empty()
}

#[test]
fn failed_finish_restores_all_fields_for_later_abort() -> Result<(), StructuralValueError> {
    let product_type = value_type(77, 78, StructuralKind::Product)?;
    let i64_type = value_type(79, 80, StructuralKind::I64)?;
    let limits = StructuralValueRuntimeLimits {
        max_tree_nodes: 1,
        ..StructuralValueRuntimeLimits::default()
    };
    let mut runtime = StructuralValueRuntime::new(limits)?;
    let destination = runtime.begin_product(product_type, vec![i64_type])?;
    runtime
        .initialize_node(
            destination,
            0,
            SemanticValue::new(
                i64_type,
                SemanticPayload::Inline(InlineStructuralValue::I64(1)),
            ),
        )
        .map_err(|failure| failure.error)?;
    assert_eq!(
        runtime.finish_destination(destination),
        Err(StructuralValueError::LimitExceeded(
            StructuralValueLimit::TreeNodes
        ))
    );
    assert_eq!(runtime.metrics().live_destinations, 1);
    let report = runtime.abort_destination(destination)?;
    assert_eq!(report.cleanup_order, vec![0]);
    assert_eq!(report.nodes_released, 1);
    runtime.verify_empty()
}

#[test]
fn abort_releases_initialized_fields_in_reverse_order_with_exact_work(
) -> Result<(), StructuralValueError> {
    let outer_type = value_type(77, 78, StructuralKind::Product)?;
    let inner_type = value_type(79, 80, StructuralKind::Product)?;
    let string_type = value_type(81, 82, StructuralKind::String)?;
    let i64_type = value_type(83, 84, StructuralKind::I64)?;
    let mut runtime = runtime()?;
    let destination = runtime.begin_product(outer_type, vec![string_type, inner_type])?;
    let string = SemanticValue::new(string_type, SemanticPayload::String(b"abc".to_vec()));
    let inner = SemanticValue::new(
        inner_type,
        SemanticPayload::Product(
            vec![
                SemanticValue::new(
                    i64_type,
                    SemanticPayload::Inline(InlineStructuralValue::I64(1)),
                ),
                SemanticValue::new(
                    i64_type,
                    SemanticPayload::Inline(InlineStructuralValue::I64(2)),
                ),
            ]
            .into(),
        ),
    );
    runtime
        .initialize_node(destination, 0, string)
        .map_err(|failure| failure.error)?;
    runtime
        .initialize_node(destination, 1, inner)
        .map_err(|failure| failure.error)?;
    let report = runtime.abort_destination(destination)?;
    assert_eq!(report.sequence, 1);
    assert_eq!(report.initialized_fields, 2);
    assert_eq!(report.cleanup_order, vec![1, 0]);
    assert_eq!(report.nodes_released, 4);
    assert_eq!(report.bytes_released, 3);
    assert_eq!(runtime.cleanup_reports().len(), 1);
    assert_eq!(runtime.metrics().destinations_aborted, 1);
    assert_eq!(runtime.metrics().destination_cleanup_work, 4);
    assert_eq!(runtime.metrics().releases, 2);
    assert_eq!(runtime.metrics().release_work, 4);
    runtime.verify_empty()
}
