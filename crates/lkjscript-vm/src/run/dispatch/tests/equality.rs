use super::*;

#[test]
fn value_equality_is_exact_and_category_checked() {
    let mut vm = test_vm();

    assert!(compare(&mut vm, Op::EqualValue, Value::UNIT, Value::UNIT));
    assert!(!compare(&mut vm, Op::EqualValue, Value::TRUE, Value::FALSE));
    let wide_left = test_i64(&mut vm, i64::MAX);
    let wide_right = test_i64(&mut vm, i64::MAX);
    assert!(compare(&mut vm, Op::EqualValue, wide_left, wide_right));

    let positive_zero = test_alloc(&mut vm, HeapObj::Float(0.0));
    let negative_zero = test_alloc(&mut vm, HeapObj::Float(-0.0));
    assert!(compare(
        &mut vm,
        Op::EqualValue,
        positive_zero,
        negative_zero
    ));
    let nan_left = test_alloc(&mut vm, HeapObj::Float(f64::NAN));
    let nan_right = test_alloc(&mut vm, HeapObj::Float(f64::NAN));
    assert!(!compare(&mut vm, Op::EqualValue, nan_left, nan_right));

    let text_left = test_alloc(&mut vm, HeapObj::Str("same".into()));
    let text_right = test_alloc(&mut vm, HeapObj::Str("same".into()));
    assert!(compare(&mut vm, Op::EqualValue, text_left, text_right));
    let symbol_left = test_alloc(&mut vm, HeapObj::Symbol("same".into()));
    let symbol_right = test_alloc(&mut vm, HeapObj::Symbol("same".into()));
    assert!(compare(&mut vm, Op::EqualValue, symbol_left, symbol_right));
    vm.push(text_left);
    vm.push(symbol_left);
    assert!(dispatch(&mut vm, Op::EqualValue as u8).is_err());
}
#[test]
fn option_and_result_value_equality_is_structural() {
    let mut vm = test_vm();
    assert!(compare(&mut vm, Op::EqualValue, Value::NONE, Value::NONE));

    let one_left = test_i64(&mut vm, 1);
    let one_right = test_i64(&mut vm, 1);
    let some_left = test_alloc(&mut vm, HeapObj::OptionSome(one_left));
    let some_right = test_alloc(&mut vm, HeapObj::OptionSome(one_right));
    assert!(compare(&mut vm, Op::EqualValue, some_left, some_right));
    assert!(!compare(&mut vm, Op::EqualValue, Value::NONE, some_left));

    let ok_left = test_alloc(&mut vm, HeapObj::ResultOk(one_left));
    let ok_right = test_alloc(&mut vm, HeapObj::ResultOk(one_right));
    let err = test_alloc(&mut vm, HeapObj::ResultErr(one_right));
    assert!(compare(&mut vm, Op::EqualValue, ok_left, ok_right));
    assert!(!compare(&mut vm, Op::EqualValue, ok_left, err));

    let mut deep_left = one_left;
    let mut deep_right = one_right;
    for _ in 0..10_000 {
        deep_left = test_alloc(&mut vm, HeapObj::OptionSome(deep_left));
        deep_right = test_alloc(&mut vm, HeapObj::OptionSome(deep_right));
    }
    assert!(compare(&mut vm, Op::EqualValue, deep_left, deep_right));

    let mut result_left = one_left;
    let mut result_right = one_right;
    for _ in 0..10_000 {
        result_left = test_alloc(&mut vm, HeapObj::ResultOk(result_left));
        result_right = test_alloc(&mut vm, HeapObj::ResultOk(result_right));
    }
    assert!(compare(&mut vm, Op::EqualValue, result_left, result_right));
}
#[test]
fn f64_bit_equality_distinguishes_signed_zero_and_accepts_equal_nan_bits() {
    let mut vm = test_vm();
    let positive_zero = test_alloc(&mut vm, HeapObj::Float(0.0));
    let negative_zero = test_alloc(&mut vm, HeapObj::Float(-0.0));
    assert!(!compare(
        &mut vm,
        Op::F64BitsEqual,
        positive_zero,
        negative_zero
    ));
    let bits = 0x7ff8_0000_0000_0042_u64;
    let nan_left = test_alloc(&mut vm, HeapObj::Float(f64::from_bits(bits)));
    let nan_right = test_alloc(&mut vm, HeapObj::Float(f64::from_bits(bits)));
    assert!(compare(&mut vm, Op::F64BitsEqual, nan_left, nan_right));
    let different_nan = test_alloc(
        &mut vm,
        HeapObj::Float(f64::from_bits(bits.wrapping_add(1))),
    );
    assert!(!compare(&mut vm, Op::F64BitsEqual, nan_left, different_nan));
}
