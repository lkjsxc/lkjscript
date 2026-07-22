//! Exact I64 and IEEE-754 F64 helpers for the VM.

use lkjscript_core::{Error, HeapObj, JitHook, Result, Value};

use crate::run::Vm;

#[derive(Clone, Copy)]
enum Number {
    I64(i64),
    F64(f64),
}

#[derive(Clone, Copy)]
pub enum Arithmetic {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy)]
pub enum Ordering {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

fn number<J: JitHook>(vm: &Vm<'_, J>, value: Value) -> Result<Number> {
    if let Some(number) = value.as_small_i64() {
        return Ok(Number::I64(number));
    }
    match vm.arena.get(value)? {
        HeapObj::Int(number) => Ok(Number::I64(*number)),
        HeapObj::Float(number) => Ok(Number::F64(*number)),
        _ => Err(Error::msg("expected I64 or F64")),
    }
}

fn optional_number<J: JitHook>(vm: &Vm<'_, J>, value: Value) -> Result<Option<Number>> {
    if let Some(number) = value.as_small_i64() {
        return Ok(Some(Number::I64(number)));
    }
    let Some(_) = value.as_heap() else {
        return Ok(None);
    };
    match vm.arena.get(value)? {
        HeapObj::Int(number) => Ok(Some(Number::I64(*number))),
        HeapObj::Float(number) => Ok(Some(Number::F64(*number))),
        _ => Ok(None),
    }
}

fn push_number<J: JitHook>(vm: &mut Vm<'_, J>, number: Number) {
    let value = match number {
        Number::I64(number) => vm.make_i64(number),
        Number::F64(number) => vm.arena.alloc(HeapObj::Float(number)),
    };
    vm.push(value);
}

pub fn bin_arithmetic<J: JitHook>(vm: &mut Vm<'_, J>, operation: Arithmetic) -> Result<()> {
    let right_value = vm.pop()?;
    let left_value = vm.pop()?;
    let right = number(vm, right_value)?;
    let left = number(vm, left_value)?;
    let result = match (left, right) {
        (Number::I64(left), Number::I64(right)) => {
            Number::I64(checked_i64(operation, left, right)?)
        }
        (left, right) => Number::F64(float_arithmetic(operation, into_f64(left), into_f64(right))),
    };
    push_number(vm, result);
    Ok(())
}

fn checked_i64(operation: Arithmetic, left: i64, right: i64) -> Result<i64> {
    let result = match operation {
        Arithmetic::Add => left.checked_add(right),
        Arithmetic::Subtract => left.checked_sub(right),
        Arithmetic::Multiply => left.checked_mul(right),
        Arithmetic::Divide if right == 0 => return Err(Error::msg("div: I64 division by zero")),
        Arithmetic::Divide => left.checked_div(right),
    };
    result.ok_or_else(|| Error::msg(format!("{}: I64 overflow", operation.name())))
}

fn float_arithmetic(operation: Arithmetic, left: f64, right: f64) -> f64 {
    match operation {
        Arithmetic::Add => left + right,
        Arithmetic::Subtract => left - right,
        Arithmetic::Multiply => left * right,
        Arithmetic::Divide => left / right,
    }
}

fn into_f64(number: Number) -> f64 {
    match number {
        Number::I64(number) => number as f64,
        Number::F64(number) => number,
    }
}

impl Arithmetic {
    fn name(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "div",
        }
    }
}

pub fn bin_ordering<J: JitHook>(vm: &mut Vm<'_, J>, ordering: Ordering) -> Result<()> {
    let right_value = vm.pop()?;
    let left_value = vm.pop()?;
    let right = number(vm, right_value)?;
    let left = number(vm, left_value)?;
    let result = match (left, right) {
        (Number::I64(left), Number::I64(right)) => compare_i64(ordering, left, right),
        (left, right) => compare_f64(ordering, into_f64(left), into_f64(right)),
    };
    vm.push(Value::from_bool(result));
    Ok(())
}

fn compare_i64(ordering: Ordering, left: i64, right: i64) -> bool {
    match ordering {
        Ordering::Less => left < right,
        Ordering::LessEqual => left <= right,
        Ordering::Greater => left > right,
        Ordering::GreaterEqual => left >= right,
    }
}

fn compare_f64(ordering: Ordering, left: f64, right: f64) -> bool {
    match ordering {
        Ordering::Less => left < right,
        Ordering::LessEqual => left <= right,
        Ordering::Greater => left > right,
        Ordering::GreaterEqual => left >= right,
    }
}

pub fn numeric_equal<J: JitHook>(
    vm: &Vm<'_, J>,
    left: Value,
    right: Value,
) -> Result<Option<bool>> {
    let left = optional_number(vm, left)?;
    let right = optional_number(vm, right)?;
    Ok(match (left, right) {
        (None, None) => None,
        (Some(_), None) | (None, Some(_)) => Some(false),
        (Some(Number::I64(left)), Some(Number::I64(right))) => Some(left == right),
        (Some(left), Some(right)) => Some(into_f64(left) == into_f64(right)),
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use lkjscript_core::{Chunk, HeapObj, NullJit, Op, MAX_SMALL_I64, MIN_SMALL_I64};

    use super::{bin_arithmetic, bin_ordering, numeric_equal, Arithmetic, Ordering};
    use crate::run::Vm;

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
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());
        for number in [
            i64::MIN,
            MIN_SMALL_I64 - 1,
            MIN_SMALL_I64,
            MAX_SMALL_I64,
            MAX_SMALL_I64 + 1,
            i64::MAX,
        ] {
            let value = vm.make_i64(number);
            assert_eq!(vm.as_i64(value).ok(), Some(number));
        }
        let wide = vm.make_i64(i64::MAX);
        assert!(wide.as_small_i64().is_none());
        assert!(matches!(vm.arena.get(wide), Ok(HeapObj::Int(i64::MAX))));
    }

    #[test]
    fn i64_arithmetic_is_exact_checked_and_truncating() {
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());

        let left = vm.make_i64(9_007_199_254_740_993);
        let right = vm.make_i64(2);
        vm.push(left);
        vm.push(right);
        bin_arithmetic(&mut vm, Arithmetic::Add).expect("exact add");
        assert_eq!(pop_i64(&mut vm), 9_007_199_254_740_995);

        for (left, right, expected) in [(7, 2, 3), (-7, 2, -3), (7, -2, -3)] {
            let left = vm.make_i64(left);
            let right = vm.make_i64(right);
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
            let left = vm.make_i64(left);
            let right = vm.make_i64(right);
            vm.push(left);
            vm.push(right);
            assert!(bin_arithmetic(&mut vm, operation).is_err());
        }
    }

    #[test]
    fn mixed_and_f64_arithmetic_preserve_ieee_f64_identity() {
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());

        let integer = vm.make_i64(1);
        let float = vm.arena.alloc(HeapObj::Float(2.0));
        vm.push(integer);
        vm.push(float);
        bin_arithmetic(&mut vm, Arithmetic::Add).expect("mixed add");
        assert_eq!(pop_f64(&mut vm), 3.0);

        let negative_zero = vm.arena.alloc(HeapObj::Float(-0.0));
        let negative_zero_again = vm.arena.alloc(HeapObj::Float(-0.0));
        vm.push(negative_zero);
        vm.push(negative_zero_again);
        bin_arithmetic(&mut vm, Arithmetic::Add).expect("signed zero add");
        assert!(pop_f64(&mut vm).is_sign_negative());

        let one = vm.arena.alloc(HeapObj::Float(1.0));
        let zero = vm.arena.alloc(HeapObj::Float(0.0));
        vm.push(one);
        vm.push(zero);
        bin_arithmetic(&mut vm, Arithmetic::Divide).expect("IEEE divide");
        assert_eq!(pop_f64(&mut vm), f64::INFINITY);
    }

    #[test]
    fn numeric_equality_and_ordering_are_exact_or_ieee() {
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());
        let left = vm.make_i64(9_007_199_254_740_992);
        let right = vm.make_i64(9_007_199_254_740_993);
        assert_eq!(numeric_equal(&vm, left, right).ok(), Some(Some(false)));

        vm.push(left);
        vm.push(right);
        bin_ordering(&mut vm, Ordering::Less).expect("exact I64 ordering");
        assert_eq!(
            vm.pop().expect("comparison result on stack").as_bool(),
            Some(true)
        );

        let close_left = vm.arena.alloc(HeapObj::Float(1.0));
        let close_right = vm.arena.alloc(HeapObj::Float(1.0 + 5.0e-13));
        assert_eq!(
            numeric_equal(&vm, close_left, close_right).ok(),
            Some(Some(false))
        );
        let nan = vm.arena.alloc(HeapObj::Float(f64::NAN));
        assert_eq!(numeric_equal(&vm, nan, nan).ok(), Some(Some(false)));
        let positive_zero = vm.arena.alloc(HeapObj::Float(0.0));
        let negative_zero = vm.arena.alloc(HeapObj::Float(-0.0));
        assert_eq!(
            numeric_equal(&vm, positive_zero, negative_zero).ok(),
            Some(Some(true))
        );
    }

    #[test]
    fn bitwise_dispatch_uses_all_i64_bits() {
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());
        let left = vm.make_i64(i64::MIN);
        let right = vm.make_i64(-1);
        vm.push(left);
        vm.push(right);
        crate::run::dispatch::dispatch(&mut vm, Op::BitXor as u8).expect("bit xor");
        assert_eq!(pop_i64(&mut vm), i64::MAX);
    }
}
