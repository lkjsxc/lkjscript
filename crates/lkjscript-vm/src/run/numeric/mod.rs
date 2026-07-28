pub(super) mod conversion;

fn bin_bits<J: RuntimeTier>(vm: &mut Vm<'_, J>, f: fn(i64, i64) -> i64) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let right = vm
        .as_i64(right)
        .map_err(|_| Error::msg("bit op expects I64"))?;
    let left = vm
        .as_i64(left)
        .map_err(|_| Error::msg("bit op expects I64"))?;
    let result = Value::from_i64(f(left, right));
    vm.push(result);
    Ok(())
}

use lkjscript_core::Op;

pub(super) fn handles(op: u8) -> bool {
    op == Op::Add as u8
        || op == Op::Sub as u8
        || op == Op::Mul as u8
        || op == Op::Div as u8
        || op == Op::EqualValue as u8
        || op == Op::Lt as u8
        || op == Op::Le as u8
        || op == Op::Gt as u8
        || op == Op::Ge as u8
        || op == Op::Not as u8
        || op == Op::BitAnd as u8
        || op == Op::BitOr as u8
        || op == Op::BitXor as u8
        || op == Op::F64BitsEqual as u8
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match op {
        x if x == Op::Add as u8 => bin_arithmetic(vm, Arithmetic::Add),
        x if x == Op::Sub as u8 => bin_arithmetic(vm, Arithmetic::Subtract),
        x if x == Op::Mul as u8 => bin_arithmetic(vm, Arithmetic::Multiply),
        x if x == Op::Div as u8 => bin_arithmetic(vm, Arithmetic::Divide),
        x if x == Op::EqualValue as u8 => super::data::equal_value(vm),
        x if x == Op::Lt as u8 => bin_ordering(vm, Ordering::Less),
        x if x == Op::Le as u8 => bin_ordering(vm, Ordering::LessEqual),
        x if x == Op::Gt as u8 => bin_ordering(vm, Ordering::Greater),
        x if x == Op::Ge as u8 => bin_ordering(vm, Ordering::GreaterEqual),
        x if x == Op::Not as u8 => {
            let value = vm.pop()?;
            let value = value
                .as_bool()
                .ok_or_else(|| Error::msg("not expects Bool"))?;
            vm.push(Value::from_bool(!value));
            Ok(())
        }
        x if x == Op::BitAnd as u8 => bin_bits(vm, |a, b| a & b),
        x if x == Op::BitOr as u8 => bin_bits(vm, |a, b| a | b),
        x if x == Op::BitXor as u8 => bin_bits(vm, |a, b| a ^ b),
        x if x == Op::F64BitsEqual as u8 => super::data::f64_bits_equal(vm),
        _ => unreachable!("opcode family checked"),
    }
}

// Exact I64 and IEEE-754 F64 helpers for the VM.

use lkjscript_core::{Error, Result, Value};

use crate::run::{RuntimeTier, Vm};

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

fn number<J: RuntimeTier>(_vm: &Vm<'_, J>, value: Value) -> Result<Number> {
    if let Some(number) = value.as_i64() {
        return Ok(Number::I64(number));
    }
    if let Some(number) = value.as_f64() {
        return Ok(Number::F64(number));
    }
    Err(Error::msg("expected I64 or F64"))
}

fn push_number<J: RuntimeTier>(vm: &mut Vm<'_, J>, number: Number) -> Result<()> {
    let value = match number {
        Number::I64(number) => Value::from_i64(number),
        Number::F64(number) => Value::from_f64_bits(number.to_bits()),
    };
    vm.push(value);
    Ok(())
}

pub fn bin_arithmetic<J: RuntimeTier>(vm: &mut Vm<'_, J>, operation: Arithmetic) -> Result<()> {
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
    push_number(vm, result)
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
            Self::Add => "add",
            Self::Subtract => "subtract",
            Self::Multiply => "multiply",
            Self::Divide => "divide",
        }
    }
}

pub fn bin_ordering<J: RuntimeTier>(vm: &mut Vm<'_, J>, ordering: Ordering) -> Result<()> {
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests;
