use lkjscript_core::{
    SemanticPayload, SemanticValue, StructuralKind, StructuralRootTableError, StructuralValueError,
};

use super::support::{publish, publish_failure, runtime, value_type};

#[test]
fn strings_and_paths_publish_with_exact_typed_access() -> Result<(), StructuralValueError> {
    let string_type = value_type(11, 12, StructuralKind::String)?;
    let path_type = value_type(13, 14, StructuralKind::Path)?;
    let mut runtime = runtime()?;
    let string = SemanticValue::new(
        string_type,
        SemanticPayload::String("héllo".as_bytes().to_vec()),
    );
    let path = SemanticValue::new(path_type, SemanticPayload::Path(b"/tmp/value".to_vec()));
    let string_key = publish(&mut runtime, string.clone())?;
    let path_key = publish(&mut runtime, path.clone())?;

    assert_eq!(runtime.value(string_key, string_type)?, &string);
    assert_eq!(
        runtime.value(string_key, string_type)?.utf8(),
        Some("héllo")
    );
    assert_eq!(runtime.value(path_key, path_type)?, &path);
    assert_eq!(
        runtime.value(path_key, path_type)?.path_bytes(),
        Some(&b"/tmp/value"[..])
    );
    assert!(runtime.path_equals(path_key, path_key, path_type)?);

    runtime.drop_owned(path_key, path_type)?;
    runtime.drop_owned(string_key, string_type)?;
    runtime.verify_empty()
}

#[test]
fn access_rejects_foreign_runtime_layout_semantic_type_and_kind() -> Result<(), StructuralValueError>
{
    let string_type = value_type(21, 22, StructuralKind::String)?;
    let wrong_layout = value_type(23, 22, StructuralKind::String)?;
    let wrong_semantic = value_type(21, 24, StructuralKind::String)?;
    let wrong_kind = value_type(21, 22, StructuralKind::I64)?;
    let mut owner = runtime()?;
    let foreign = runtime()?;
    assert_ne!(owner.identity(), foreign.identity());
    let key = publish(
        &mut owner,
        SemanticValue::new(string_type, SemanticPayload::String(b"typed".to_vec())),
    )?;

    assert_eq!(
        foreign.value(key, string_type),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::StaleRoot
        ))
    );
    assert_eq!(
        owner.value(key, wrong_layout),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::WrongLayout
        ))
    );
    assert_eq!(
        owner.value(key, wrong_semantic),
        Err(StructuralValueError::RootTable(
            StructuralRootTableError::WrongSemanticType
        ))
    );
    assert_eq!(
        owner.value(key, wrong_kind),
        Err(StructuralValueError::WrongPayloadKind)
    );
    owner.drop_owned(key, string_type)?;
    owner.verify_empty()?;
    foreign.verify_empty()
}

#[test]
fn invalid_text_path_and_payload_kind_return_the_original_value() -> Result<(), StructuralValueError>
{
    let string_type = value_type(31, 32, StructuralKind::String)?;
    let path_type = value_type(33, 34, StructuralKind::Path)?;
    let mut runtime = runtime()?;
    let invalid_utf8 = SemanticValue::new(
        string_type,
        SemanticPayload::String(vec![0xf0, 0x28, 0x8c, 0x28]),
    );
    let invalid_path = SemanticValue::new(path_type, SemanticPayload::Path(b"relative".to_vec()));
    let wrong_payload = SemanticValue::new(string_type, SemanticPayload::Path(b"/path".to_vec()));

    let failure = publish_failure(runtime.publish_owned(invalid_utf8.clone()))?;
    assert_eq!(failure.error, StructuralValueError::InvalidUtf8);
    assert_eq!(failure.value, invalid_utf8);
    let failure = publish_failure(runtime.publish_owned(invalid_path.clone()))?;
    assert_eq!(failure.error, StructuralValueError::InvalidPath);
    assert_eq!(failure.value, invalid_path);
    let failure = publish_failure(runtime.publish_owned(wrong_payload.clone()))?;
    assert_eq!(failure.error, StructuralValueError::WrongPayloadKind);
    assert_eq!(failure.value, wrong_payload);
    assert_eq!(runtime.metrics().live_objects, 0);
    runtime.verify_empty()
}
