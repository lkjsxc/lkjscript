use super::{list_values_equal, EvalValue, Flow};

#[test]
#[ignore = "release stress crosses the former one-million-element ceiling"]
fn evaluator_list_equality_completes_beyond_former_limit() {
    let values = std::iter::repeat_with(|| EvalValue::Unit)
        .take(1_000_001)
        .collect::<Vec<_>>();
    assert_eq!(list_values_equal(&values, &values).ok(), Some(true));
}

#[test]
fn evaluator_list_equality_is_incremental_at_difference_and_length_boundaries() {
    let left = [EvalValue::I64(1), EvalValue::Unit, EvalValue::Unit];
    let different_head = [EvalValue::I64(2), EvalValue::Unit, EvalValue::Unit];
    assert_eq!(list_values_equal(&left, &different_head).ok(), Some(false));
    assert_eq!(list_values_equal(&left[..2], &left).ok(), Some(false));
    assert_eq!(list_values_equal(&left, &left).ok(), Some(true));
}

#[test]
fn evaluator_list_equality_propagates_element_comparison_errors(
) -> Result<(), lkjscript_core::InvalidUniqueKeyWord> {
    let owner = lkjscript_core::UniqueKeyWord::new((1_u64 << 32) | 1)?;
    let vector = EvalValue::ByteVector(owner);
    assert!(matches!(
        list_values_equal(&[vector], &[EvalValue::Unit]),
        Err(Flow::Trap(message)) if message == "equal-value category mismatch"
    ));
    Ok(())
}
