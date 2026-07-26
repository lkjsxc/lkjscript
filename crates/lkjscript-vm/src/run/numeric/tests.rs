use lkjscript_core::{ExecutionConfig, HeapObj, Op, MAX_SMALL_I64, MIN_SMALL_I64};

use crate::run::NoTier as NullJit;

use super::{bin_arithmetic, bin_ordering, Arithmetic, Ordering};
use crate::run::{test_chunk, Vm};

fn test_vm() -> Vm<'static, NullJit> {
    let chunk = Box::leak(Box::new(test_chunk()));
    Vm::new(
        chunk,
        NullJit,
        crate::ExecutionInputs::default(),
        ExecutionConfig::default(),
    )
}

fn test_i64(vm: &mut Vm<'_, NullJit>, number: i64) -> lkjscript_core::Value {
    vm.make_i64(number).expect("test I64 allocation")
}

fn test_float(vm: &mut Vm<'_, NullJit>, number: f64) -> lkjscript_core::Value {
    vm.arena
        .alloc(HeapObj::Float(number))
        .expect("test F64 allocation")
}

fn pop_i64(vm: &mut Vm<'_, NullJit>) -> i64 {
    let value = vm.pop().expect("numeric result on stack");
    vm.as_i64(value).expect("I64 result")
}

fn pop_f64(vm: &mut Vm<'_, NullJit>) -> f64 {
    let value = vm.pop().expect("numeric result on stack");
    let HeapObj::Float(number) = vm.arena.get(value).expect("F64 result") else {
        panic!("expected F64 result");
    };
    *number
}

#[test]
fn complete_i64_range_round_trips_through_immediate_and_boxed_values() {
    let mut vm = test_vm();
    for number in [
        i64::MIN,
        MIN_SMALL_I64 - 1,
        MIN_SMALL_I64,
        MAX_SMALL_I64,
        MAX_SMALL_I64 + 1,
        i64::MAX,
    ] {
        let value = test_i64(&mut vm, number);
        assert_eq!(vm.as_i64(value).ok(), Some(number));
    }
    let wide = test_i64(&mut vm, i64::MAX);
    assert!(wide.as_small_i64().is_none());
    assert!(matches!(vm.arena.get(wide), Ok(HeapObj::Int(i64::MAX))));
}
#[test]
fn i64_arithmetic_is_exact_checked_and_truncating() {
    let mut vm = test_vm();

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
fn mixed_and_f64_arithmetic_preserve_ieee_f64_identity() {
    let mut vm = test_vm();

    let integer = test_i64(&mut vm, 1);
    let float = test_float(&mut vm, 2.0);
    vm.push(integer);
    vm.push(float);
    bin_arithmetic(&mut vm, Arithmetic::Add).expect("mixed add");
    assert_eq!(pop_f64(&mut vm), 3.0);

    let negative_zero = test_float(&mut vm, -0.0);
    let negative_zero_again = test_float(&mut vm, -0.0);
    vm.push(negative_zero);
    vm.push(negative_zero_again);
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
    let mut vm = test_vm();
    let left = test_i64(&mut vm, 9_007_199_254_740_992);
    let right = test_i64(&mut vm, 9_007_199_254_740_993);
    vm.push(left);
    vm.push(right);
    bin_ordering(&mut vm, Ordering::Less).expect("exact I64 ordering");
    assert_eq!(
        vm.pop().expect("comparison result on stack").as_bool(),
        Some(true)
    );

    let integer = test_i64(&mut vm, 1);
    let float = test_float(&mut vm, 1.5);
    vm.push(integer);
    vm.push(float);
    bin_ordering(&mut vm, Ordering::Less).expect("mixed numeric ordering");
    assert_eq!(
        vm.pop().expect("comparison result on stack").as_bool(),
        Some(true)
    );
}
#[test]
fn bitwise_dispatch_uses_all_i64_bits() {
    let mut vm = test_vm();
    let left = test_i64(&mut vm, i64::MIN);
    let right = test_i64(&mut vm, -1);
    vm.push(left);
    vm.push(right);
    crate::run::dispatch::dispatch(&mut vm, Op::BitXor as u8).expect("bit xor");
    assert_eq!(pop_i64(&mut vm), i64::MAX);
}
