use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralKind,
    StructuralRootTableError, StructuralValueError, Value,
};

use super::support::{runtime, value_type};

#[test]
fn enum_active_payload_moves_into_nested_product_and_rejects_legacy_traced(
) -> Result<(), StructuralValueError> {
    let enum_type = value_type(91, 92, StructuralKind::Enum)?;
    let product_type = value_type(93, 94, StructuralKind::Product)?;
    let string_type = value_type(95, 96, StructuralKind::String)?;
    let i64_type = value_type(97, 98, StructuralKind::I64)?;
    let mut runtime = runtime()?;

    let enum_destination = runtime.begin_enum(enum_type, 7, vec![string_type])?;
    let text = SemanticValue::new(string_type, SemanticPayload::String(b"active".to_vec()));
    runtime
        .initialize_node(enum_destination, 0, text.clone())
        .map_err(|failure| failure.error)?;
    let enum_value = runtime.finish_destination_value(enum_destination)?;
    let enum_key = enum_value
        .as_structural_root()
        .ok_or(StructuralValueError::InvariantViolation)?;

    let product_destination = runtime.begin_product(product_type, vec![enum_type, i64_type])?;
    assert_eq!(
        runtime.initialize_value(product_destination, 1, Value::from_legacy_traced(4)),
        Err(StructuralValueError::MixedValue)
    );
    assert_eq!(
        runtime.finish_destination(product_destination),
        Err(StructuralValueError::IncompleteDestination)
    );
    runtime.initialize_value(product_destination, 0, enum_value)?;
    assert_eq!(
        runtime.value(enum_key, enum_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::MovedRoot
        ))
    );
    runtime.initialize_value(product_destination, 1, Value::from_i64(17))?;
    let nested_key = runtime.finish_destination(product_destination)?;
    let integer = SemanticValue::new(
        i64_type,
        SemanticPayload::Inline(InlineStructuralValue::I64(17)),
    );
    let expected_enum = SemanticValue::new(
        enum_type,
        SemanticPayload::Enum {
            tag: 7,
            active_payload: vec![text],
        },
    );
    let expected = SemanticValue::new(
        product_type,
        SemanticPayload::Product(vec![expected_enum, integer]),
    );
    assert_eq!(runtime.value(nested_key, product_type)?, &expected);
    let exported = runtime.export_semantic(nested_key, product_type)?;
    assert_eq!(exported, expected);
    assert_eq!(runtime.metrics().live_objects, 0);
    runtime.verify_empty()
}
