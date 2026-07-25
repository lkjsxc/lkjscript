use super::*;

#[test]
fn list_equality_is_structural_bounded_and_rejects_improper_lists() {
    let mut vm = test_vm();
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
        list_values_equal(&vm.arena, shorter, one_again, 1).ok(),
        Some(true)
    );
    assert!(list_values_equal(&vm.arena, first, same, 1).is_err());

    let improper_car = test_i64(&mut vm, 1);
    let improper_cdr = test_i64(&mut vm, 2);
    let improper = test_alloc(
        &mut vm,
        HeapObj::Pair {
            car: improper_car,
            cdr: improper_cdr,
        },
    );
    vm.push(improper);
    vm.push(first);
    assert!(dispatch(&mut vm, Op::ListEqual as u8).is_err());
    vm.push(Value::EMPTY_LIST);
    vm.push(improper_cdr);
    assert!(dispatch(&mut vm, Op::ListEqual as u8).is_err());
    let one = i64_list(&mut vm, &[1]);
    vm.push(one);
    vm.push(improper);
    assert!(dispatch(&mut vm, Op::ListEqual as u8).is_err());
}
#[test]
fn list_equality_accepts_exact_global_limit_and_rejects_one_more() {
    let mut vm = test_vm();
    let mut at_limit = Value::EMPTY_LIST;
    for _ in 0..MAX_LIST_EQUAL_STEPS {
        at_limit = test_alloc(
            &mut vm,
            HeapObj::Pair {
                car: Value::UNIT,
                cdr: at_limit,
            },
        );
    }
    assert_eq!(
        list_values_equal(&vm.arena, at_limit, at_limit, MAX_LIST_EQUAL_STEPS).ok(),
        Some(true)
    );
    let over_limit = test_alloc(
        &mut vm,
        HeapObj::Pair {
            car: Value::UNIT,
            cdr: at_limit,
        },
    );
    assert!(list_values_equal(&vm.arena, over_limit, over_limit, MAX_LIST_EQUAL_STEPS).is_err());
}
