use super::*;

#[test]
fn language_results_preserve_operation_error_text() {
    let mut arena = Arena::default();
    let result = language_result(&mut arena, Err(Error::msg("sys-example: failure")))
        .expect("language error allocation");
    assert_eq!(is_ok(&arena, result).ok(), Some(Value::FALSE));
    let error = unwrap_err(&arena, result).expect("unwrap Result error");
    assert_eq!(as_str(&arena, error).ok(), Some("sys-example: failure"));
    let unwrapped = unwrap_ok(&arena, result)
        .err()
        .map(|error| error.to_string());
    assert_eq!(
        unwrapped.as_deref(),
        Some("unwrap-ok: sys-example: failure")
    );
}
#[test]
fn option_variants_are_distinct_and_type_checked() {
    let mut arena = Arena::default();
    assert_eq!(is_some(&arena, Value::NONE).ok(), Some(Value::FALSE));
    assert!(unwrap_some(&arena, Value::NONE)
        .expect_err("none must not unwrap")
        .to_string()
        .contains("unwrap-some on none"));

    let payload = Value::from_small_i64(7).expect("7 is an immediate I64");
    let some = option_some(&mut arena, payload).expect("option allocation");
    assert_eq!(is_some(&arena, some).ok(), Some(Value::TRUE));
    assert_eq!(unwrap_some(&arena, some).ok(), Some(payload));
    assert!(is_some(&arena, Value::UNIT).is_err());
    assert!(unwrap_some(&arena, Value::EMPTY_LIST).is_err());
}
#[test]
fn numeric_string_conversions_are_type_strict_and_exact() {
    let mut arena = Arena::default();
    let text = str_from_i64(&mut arena, i64::MIN).expect("integer string allocation");
    assert_eq!(as_str(&arena, text).ok(), Some("-9223372036854775808"));

    let integer = Value::from_small_i64(2).expect("2 is an immediate I64");
    assert!(str_from_f64(&mut arena, integer).is_err());
    let float = arena
        .alloc(HeapObj::Float(2.0))
        .expect("test float allocation");
    let text = str_from_f64(&mut arena, float).expect("format F64");
    assert_eq!(as_str(&arena, text).ok(), Some("2"));
}
