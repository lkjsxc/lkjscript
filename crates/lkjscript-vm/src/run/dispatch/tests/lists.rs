use super::*;

#[test]
fn list_equality_is_segmented_complete_and_rejects_forged_handles() {
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
    assert_eq!(list_values_equal(&mut vm, shorter, one_again), Ok(true));

    let forged = Value::from_segmented_list(u64::MAX);
    vm.push(forged);
    vm.push(first);
    assert!(dispatch(&mut vm, Op::ListEqual as u8).is_err());
    vm.push(Value::EMPTY_LIST);
    vm.push(Value::UNIT);
    assert!(dispatch(&mut vm, Op::ListEqual as u8).is_err());
}
#[test]
#[ignore = "release stress crosses the former one-million-element ceiling"]
fn list_equality_executes_beyond_former_limit() {
    test_vm!(vm);
    let mut left = Value::EMPTY_LIST;
    let mut right = Value::EMPTY_LIST;
    for _ in 0..1_000_001 {
        left = vm
            .list_prepend(Value::UNIT, left)
            .expect("prepend left stress element");
        right = vm
            .list_prepend(Value::UNIT, right)
            .expect("prepend right stress element");
    }
    vm.push(left);
    vm.push(right);
    dispatch(&mut vm, Op::ListEqual as u8).expect("execute complete list equality");
    assert_eq!(vm.pop().ok().and_then(Value::as_bool), Some(true));
}
