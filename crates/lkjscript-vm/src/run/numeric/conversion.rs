use super::super::*;
use lkjscript_core::NumericError;

pub(in crate::run) fn handles(op: u8) -> bool {
    matches!(
        Op::from_byte(op),
        Some(
            Op::F64FromI64Exact | Op::F64FromI64Rounded | Op::I64FromF64Exact | Op::I64FromF64Trunc
        )
    )
}

pub(in crate::run) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match Op::from_byte(op) {
        Some(Op::F64FromI64Exact) => {
            let input = pop_i64(vm)?;
            let result = lkjscript_core::f64_from_i64_exact(input);
            push_f64_result(vm, result)
        }
        Some(Op::F64FromI64Rounded) => {
            let input = pop_i64(vm)?;
            let value = Value::from_f64_bits(lkjscript_core::f64_from_i64_rounded(input).to_bits());
            vm.push(value);
            Ok(())
        }
        Some(Op::I64FromF64Exact) => {
            let input = pop_f64(vm)?;
            let result = lkjscript_core::i64_from_f64_exact(input);
            push_i64_result(vm, result)
        }
        Some(Op::I64FromF64Trunc) => {
            let input = pop_f64(vm)?;
            let result = lkjscript_core::i64_from_f64_trunc(input);
            push_i64_result(vm, result)
        }
        _ => unreachable!("numeric conversion opcode family checked"),
    }
}

fn pop_i64<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<i64> {
    let value = vm.pop()?;
    vm.as_i64(value)
        .map_err(|_| Error::msg("I64-to-F64 conversion expects I64"))
}

fn pop_f64<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<f64> {
    let value = vm.pop()?;
    vm.as_f64(value)
        .map_err(|_| Error::msg("F64-to-I64 conversion expects F64"))
}

fn push_f64_result<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    result: std::result::Result<f64, NumericError>,
) -> Result<()> {
    let result = match result {
        Ok(value) => structural_ops::publish_numeric_result(
            vm,
            structural_ops::HostValueType::F64,
            Ok(structural_ops::HostValue::F64Bits(value.to_bits())),
        )?,
        Err(error) => structural_ops::publish_numeric_result(
            vm,
            structural_ops::HostValueType::F64,
            Err(error),
        )?,
    };
    vm.push(result);
    Ok(())
}

fn push_i64_result<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    result: std::result::Result<i64, NumericError>,
) -> Result<()> {
    let result = match result {
        Ok(value) => structural_ops::publish_numeric_result(
            vm,
            structural_ops::HostValueType::I64,
            Ok(structural_ops::HostValue::I64(value)),
        )?,
        Err(error) => structural_ops::publish_numeric_result(
            vm,
            structural_ops::HostValueType::I64,
            Err(error),
        )?,
    };
    vm.push(result);
    Ok(())
}
