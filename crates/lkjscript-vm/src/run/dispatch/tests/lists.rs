use super::*;

#[test]
fn list_equality_is_segmented_bounded_and_rejects_forged_handles() {
    test_vm!(vm);
    assert!(compare(
        &mut vm,
        Op::ListEqual,
        Value::EMPTY_LIST,
        Value::EMPTY_LIST
    ));
    let first = i64_list(&mut vm, &[1, 2]);
    let same = i64_list(&mut vm, &[1, 2]);
    let different = i64_list(&mut vm, &[1, 3]);
    let shorter = i64_list(&mut vm, &[1]);
    assert!(compare(&mut vm, Op::ListEqual, first, same));
    assert!(!compare(&mut vm, Op::ListEqual, first, different));
    assert!(!compare(&mut vm, Op::ListEqual, first, shorter));
    let one_again = i64_list(&mut vm, &[1]);
    assert_eq!(
        list_values_equal(&vm, shorter, one_again, 1).ok(),
        Some(true)
    );
    assert!(list_values_equal(&vm, first, same, 1).is_err());

    let forged = Value::from_segmented_list(u64::MAX);
    vm.push(forged);
    vm.push(first);
    assert!(dispatch(&mut vm, Op::ListEqual as u8).is_err());
    vm.push(Value::EMPTY_LIST);
    vm.push(Value::UNIT);
    assert!(dispatch(&mut vm, Op::ListEqual as u8).is_err());
}
#[test]
fn list_equality_accepts_exact_global_limit_and_rejects_one_more() {
    test_vm!(vm);
    let mut at_limit = Value::EMPTY_LIST;
    for _ in 0..MAX_LIST_EQUAL_STEPS {
        at_limit = vm
            .list_prepend(Value::UNIT, at_limit)
            .expect("prepend exact-bound list element");
    }
    assert_eq!(
        list_values_equal(&vm, at_limit, at_limit, MAX_LIST_EQUAL_STEPS).ok(),
        Some(true)
    );
    let over_limit = vm
        .list_prepend(Value::UNIT, at_limit)
        .expect("prepend over-bound list element");
    assert!(list_values_equal(&vm, over_limit, over_limit, MAX_LIST_EQUAL_STEPS,).is_err());
}
