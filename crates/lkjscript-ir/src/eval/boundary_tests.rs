use super::{list_values_equal, EvalBuffer, EvalValue, Flow};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn evaluator_list_equality_accepts_limit_and_rejects_limit_plus_one() {
    let limit = 1_000_000;
    let values = vec![EvalValue::Unit; limit + 1];
    assert_eq!(
        list_values_equal(&values[..limit], &values[..limit], limit).ok(),
        Some(true)
    );
    assert!(matches!(
        list_values_equal(&values, &values, limit),
        Err(Flow::Trap(message)) if message == "list-equal step limit exceeded"
    ));
}

#[test]
fn evaluator_list_equality_is_incremental_at_difference_and_length_boundaries() {
    let left = [EvalValue::I64(1), EvalValue::Unit, EvalValue::Unit];
    let different_head = [EvalValue::I64(2), EvalValue::Unit, EvalValue::Unit];
    assert_eq!(
        list_values_equal(&left, &different_head, 2).ok(),
        Some(false)
    );
    assert_eq!(list_values_equal(&left[..2], &left, 2).ok(), Some(false));
    assert!(list_values_equal(&left, &left, 2).is_err());
}

#[test]
fn evaluator_list_equality_propagates_element_comparison_errors() {
    let buffer = EvalValue::Buf(EvalBuffer {
        id: 1,
        bytes: Rc::new(RefCell::new(Vec::new())),
    });
    assert!(matches!(
        list_values_equal(&[buffer], &[EvalValue::Unit], 1),
        Err(Flow::Trap(message)) if message == "equal-value category mismatch"
    ));
}
