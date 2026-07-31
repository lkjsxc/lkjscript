use lkjscript_core::{
    SemanticPayload, SemanticValue, StructuralFieldPath, StructuralKind, StructuralProjection,
    StructuralRootTableError, StructuralValueError,
};

use super::support::{publish, runtime, value_type};

fn field_projection(value_type: lkjscript_core::StructuralType) -> StructuralProjection {
    StructuralProjection::Field {
        path: StructuralFieldPath::root(),
        expected: value_type,
    }
}

#[test]
fn shared_and_exclusive_views_conflict_end_and_block_drop() -> Result<(), StructuralValueError> {
    let string_type = value_type(51, 52, StructuralKind::String)?;
    let mut runtime = runtime()?;
    let key = publish(
        &mut runtime,
        SemanticValue::new(string_type, SemanticPayload::String(b"borrow".to_vec())),
    )?;
    let first = runtime.borrow_projected(key, string_type, field_projection(string_type), false)?;
    let second =
        runtime.borrow_projected(key, string_type, field_projection(string_type), false)?;
    assert_eq!(runtime.projected(first)?.utf8(), Some("borrow"));
    assert!(matches!(
        runtime.byte_vector_mut(first),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::BorrowConflict
        ))
    ));
    assert_eq!(
        runtime.borrow_projected(key, string_type, field_projection(string_type), true),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::BorrowConflict
        ))
    );
    assert_eq!(
        runtime.drop_owned(key, string_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::LiveLoan
        ))
    );

    runtime.end_view(second)?;
    runtime.end_view(first)?;
    assert_eq!(
        runtime.projected(first),
        Err(StructuralValueError::StaleView)
    );
    let exclusive =
        runtime.borrow_projected(key, string_type, field_projection(string_type), true)?;
    assert_eq!(
        runtime.borrow_projected(key, string_type, field_projection(string_type), false),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::BorrowConflict
        ))
    );
    runtime.end_view(exclusive)?;
    assert_eq!(runtime.metrics().views_created, 3);
    assert_eq!(runtime.metrics().views_ended, 3);
    assert_eq!(runtime.metrics().live_views, 0);
    runtime.drop_owned(key, string_type)?;
    runtime.verify_empty()
}

#[test]
fn utf8_views_require_valid_byte_and_character_boundaries() -> Result<(), StructuralValueError> {
    let string_type = value_type(53, 54, StructuralKind::String)?;
    let mut runtime = runtime()?;
    let key = publish(
        &mut runtime,
        SemanticValue::new(
            string_type,
            SemanticPayload::String("aéz".as_bytes().to_vec()),
        ),
    )?;
    let projection = |start, end| StructuralProjection::Utf8 {
        path: StructuralFieldPath::root(),
        expected: string_type,
        start,
        end,
    };
    let view = runtime.borrow_projected(key, string_type, projection(1, 3), false)?;
    assert_eq!(runtime.utf8_view(view)?, "é");
    runtime.end_view(view)?;
    assert_eq!(
        runtime.borrow_projected(key, string_type, projection(2, 3), false),
        Err(StructuralValueError::InvalidRange)
    );
    assert_eq!(
        runtime.borrow_projected(key, string_type, projection(1, 5), false),
        Err(StructuralValueError::InvalidRange)
    );
    assert_eq!(runtime.root_stats().live_loans, 0);
    assert_eq!(runtime.metrics().live_views, 0);
    runtime.drop_owned(key, string_type)?;
    runtime.verify_empty()
}
