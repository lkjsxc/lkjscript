use lkjscript_core::{ExecutionConfig, Op, Value};

use crate::run::NoTier as NullJit;

use super::{bin_arithmetic, bin_ordering, Arithmetic, Ordering};
use crate::run::{test_chunk, Vm};

macro_rules! test_vm {
    ($name:ident) => {
        let chunk = test_chunk();
        let mut $name = Vm::new(
            &chunk,
            NullJit,
            crate::ExecutionInputs::default(),
            ExecutionConfig::default(),
        );
    };
}

fn test_i64(_vm: &mut Vm<'_, NullJit>, number: i64) -> Value {
    Value::from_i64(number)
}

fn test_float(_vm: &mut Vm<'_, NullJit>, number: f64) -> Value {
    Value::from_f64_bits(number.to_bits())
}

fn pop_i64(vm: &mut Vm<'_, NullJit>) -> i64 {
    let value = vm.pop().expect("numeric result on stack");
    vm.as_i64(value).expect("I64 result")
}

fn pop_f64(vm: &mut Vm<'_, NullJit>) -> f64 {
    let value = vm.pop().expect("numeric result on stack");
    vm.as_f64(value).expect("F64 result")
}

#[test]
fn complete_i64_range_round_trips_inline_without_runtime_storage() {
    test_vm!(vm);
    for number in [
        i64::MIN,
        -(1_i64 << 60) - 1,
        -(1_i64 << 60),
        (1_i64 << 60) - 1,
        1_i64 << 60,
        i64::MAX,
    ] {
        let value = test_i64(&mut vm, number);
        assert_eq!(vm.as_i64(value).ok(), Some(number));
    }
}

#[test]
fn i64_arithmetic_is_exact_checked_and_allocation_free() {
    test_vm!(vm);
    let left = test_i64(&mut vm, 9_007_199_254_740_993);
    let right = test_i64(&mut vm, 2);
    vm.push(left);
    vm.push(right);
    bin_arithmetic(&mut vm, Arithmetic::Add).expect("exact add");
    assert_eq!(pop_i64(&mut vm), 9_007_199_254_740_995);

    for (left, right, expected) in [(7, 2, 3), (-7, 2, -3), (7, -2, -3)] {
        let left = test_i64(&mut vm, left);
        let right = test_i64(&mut vm, right);
        vm.push(left);
        vm.push(right);
        bin_arithmetic(&mut vm, Arithmetic::Divide).expect("I64 divide");
        assert_eq!(pop_i64(&mut vm), expected);
    }
    for (left, right, operation) in [
        (i64::MAX, 1, Arithmetic::Add),
        (i64::MIN, 1, Arithmetic::Subtract),
        (i64::MAX, 2, Arithmetic::Multiply),
        (1, 0, Arithmetic::Divide),
        (i64::MIN, -1, Arithmetic::Divide),
    ] {
        let left = test_i64(&mut vm, left);
        let right = test_i64(&mut vm, right);
        vm.push(left);
        vm.push(right);
        assert!(bin_arithmetic(&mut vm, operation).is_err());
    }
}

#[test]
fn mixed_and_f64_arithmetic_preserve_ieee_identity_without_allocation() {
    test_vm!(vm);
    let integer = test_i64(&mut vm, 1);
    let float = test_float(&mut vm, 2.0);
    vm.push(integer);
    vm.push(float);
    bin_arithmetic(&mut vm, Arithmetic::Add).expect("mixed add");
    assert_eq!(pop_f64(&mut vm), 3.0);

    let negative_zero = test_float(&mut vm, -0.0);
    vm.push(negative_zero);
    vm.push(negative_zero);
    bin_arithmetic(&mut vm, Arithmetic::Add).expect("signed zero add");
    assert!(pop_f64(&mut vm).is_sign_negative());

    let one = test_float(&mut vm, 1.0);
    let zero = test_float(&mut vm, 0.0);
    vm.push(one);
    vm.push(zero);
    bin_arithmetic(&mut vm, Arithmetic::Divide).expect("IEEE divide");
    assert_eq!(pop_f64(&mut vm), f64::INFINITY);
}

#[test]
fn numeric_ordering_is_exact_and_uses_ieee_promotion() {
    test_vm!(vm);
    let left = test_i64(&mut vm, 9_007_199_254_740_992);
    let right = test_i64(&mut vm, 9_007_199_254_740_993);
    vm.push(left);
    vm.push(right);
    bin_ordering(&mut vm, Ordering::Less).expect("exact I64 ordering");
    assert_eq!(vm.pop().expect("result").as_bool(), Some(true));

    let integer = test_i64(&mut vm, 1);
    let float = test_float(&mut vm, 1.5);
    vm.push(integer);
    vm.push(float);
    bin_ordering(&mut vm, Ordering::Less).expect("mixed numeric ordering");
    assert_eq!(vm.pop().expect("result").as_bool(), Some(true));
}

#[test]
fn bitwise_dispatch_uses_all_i64_bits_without_allocation() {
    test_vm!(vm);
    let left = test_i64(&mut vm, i64::MIN);
    let right = test_i64(&mut vm, -1);
    vm.push(left);
    vm.push(right);
    crate::run::dispatch::dispatch(&mut vm, Op::BitXor as u8).expect("bit xor");
    assert_eq!(pop_i64(&mut vm), i64::MAX);
}
