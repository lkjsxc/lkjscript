use super::*;

#[test]
fn language_results_preserve_structured_system_error_identity() {
    let mut arena = Arena::default();
    let kind = lkjscript_core::SystemErrorKind::Io;
    let result = language_result(&mut arena, kind, Err(Error::msg("sys-example: failure")))
        .expect("language error allocation");
    let error = match arena.get(result).expect("Result value") {
        HeapObj::Enum {
            layout,
            physical_tag: 1,
            active_payload,
        } if layout.bytes() == lkjscript_core::RESULT_LAYOUT => active_payload[0],
        other => panic!("malformed Result error: {other:?}"),
    };
    match arena.get(error).expect("SystemError value") {
        HeapObj::Enum {
            layout,
            physical_tag,
            active_payload,
        } => {
            assert_eq!(layout.bytes(), lkjscript_core::SYSTEM_ERROR_LAYOUT);
            assert_eq!(*physical_tag, kind.physical_tag());
            assert_eq!(active_payload.len(), 2);
        }
        other => panic!("malformed SystemError: {other:?}"),
    }
    assert!(language_result(
        &mut arena,
        lkjscript_core::SystemErrorKind::Utf8,
        Err(Error::msg("unstructured")),
    )
    .is_err());
}

#[test]
fn system_utf8_errors_preserve_both_closed_variant_identities() {
    let mut arena = Arena::default();
    let kind = lkjscript_core::Utf8ErrorKind::Surrogate;
    let result = system_utf8_error(&mut arena, lkjscript_core::Utf8Failure { offset: 2, kind })
        .expect("structured system UTF-8 allocation");
    let system = match arena.get(result).expect("Result value") {
        HeapObj::Enum {
            physical_tag: 1,
            active_payload,
            ..
        } => active_payload[0],
        other => panic!("malformed Result error: {other:?}"),
    };
    let utf8 = match arena.get(system).expect("SystemError value") {
        HeapObj::Enum {
            layout,
            physical_tag,
            active_payload,
        } => {
            assert_eq!(layout.bytes(), lkjscript_core::SYSTEM_ERROR_LAYOUT);
            assert_eq!(
                *physical_tag,
                lkjscript_core::SystemErrorKind::Utf8.physical_tag()
            );
            active_payload[0]
        }
        other => panic!("malformed SystemError: {other:?}"),
    };
    match arena.get(utf8).expect("Utf8Error value") {
        HeapObj::Enum {
            layout,
            physical_tag,
            active_payload,
        } => {
            assert_eq!(layout.bytes(), lkjscript_core::UTF8_ERROR_LAYOUT);
            assert_eq!(*physical_tag, kind.physical_tag());
            assert_eq!(active_payload[0].as_i64(), Some(2));
        }
        other => panic!("malformed Utf8Error: {other:?}"),
    }
}

#[test]
fn option_constructors_use_generic_enum_layout_and_tags() {
    let mut arena = Arena::default();
    let none = option_none(&mut arena).expect("none allocation");
    let payload = Value::from_i64(7);
    let some = option_some(&mut arena, payload).expect("some allocation");
    for (value, tag, fields) in [(none, 1, 0), (some, 0, 1)] {
        match arena.get(value).expect("Option value") {
            HeapObj::Enum {
                layout,
                physical_tag,
                active_payload,
            } => {
                assert_eq!(layout.bytes(), lkjscript_core::OPTION_LAYOUT);
                assert_eq!(*physical_tag, tag);
                assert_eq!(active_payload.len(), fields);
            }
            other => panic!("malformed Option: {other:?}"),
        }
    }
}
#[test]
fn numeric_string_conversions_are_type_strict_and_exact() {
    let mut arena = Arena::default();
    let text = str_from_i64(&mut arena, i64::MIN).expect("integer string allocation");
    assert_eq!(as_str(&arena, text).ok(), Some("-9223372036854775808"));

    let integer = Value::from_i64(2);
    assert!(str_from_f64(&mut arena, integer).is_err());
    let float = Value::from_f64_bits(2.0_f64.to_bits());
    let text = str_from_f64(&mut arena, float).expect("format F64");
    assert_eq!(as_str(&arena, text).ok(), Some("2"));
}
