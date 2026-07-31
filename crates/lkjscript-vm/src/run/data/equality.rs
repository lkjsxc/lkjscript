use super::*;

fn value_equal<J: RuntimeTier>(vm: &Vm<'_, J>, left: Value, right: Value) -> Result<bool> {
    let mut pending = vec![(left, right)];
    let mut steps = 0_usize;
    while let Some((left, right)) = pending.pop() {
        if steps == MAX_LIST_EQUAL_STEPS {
            return Err(Error::msg("equal-value step limit exceeded"));
        }
        steps += 1;
        if left.is_unit() || right.is_unit() {
            if left.is_unit() && right.is_unit() {
                continue;
            }
            return Err(Error::msg("equal-value runtime type mismatch"));
        }
        let left_bool = left.as_bool();
        let right_bool = right.as_bool();
        if left_bool.is_some() || right_bool.is_some() {
            match (left_bool, right_bool) {
                (Some(left), Some(right)) if left == right => continue,
                (Some(_), Some(_)) => return Ok(false),
                _ => return Err(Error::msg("equal-value runtime type mismatch")),
            }
        }
        let left_i64 = left.as_i64();
        let right_i64 = right.as_i64();
        if left_i64.is_some() || right_i64.is_some() {
            match (left_i64, right_i64) {
                (Some(left), Some(right)) if left == right => continue,
                (Some(_), Some(_)) => return Ok(false),
                _ => return Err(Error::msg("equal-value runtime type mismatch")),
            }
        }
        let left_f64 = left.as_f64();
        let right_f64 = right.as_f64();
        if left_f64.is_some() || right_f64.is_some() {
            match (left_f64, right_f64) {
                (Some(left), Some(right)) if left == right => continue,
                (Some(_), Some(_)) => return Ok(false),
                _ => return Err(Error::msg("equal-value runtime type mismatch")),
            }
        }
        let left_symbol = left.as_symbol();
        let right_symbol = right.as_symbol();
        if left_symbol.is_some() || right_symbol.is_some() {
            match (left_symbol, right_symbol) {
                (Some(left), Some(right)) => {
                    if symbol_text(vm.chunk, left)? == symbol_text(vm.chunk, right)? {
                        continue;
                    }
                    return Ok(false);
                }
                _ => return Err(Error::msg("equal-value runtime type mismatch")),
            }
        }
        let left_string = crate::run::structural_ops::copy_string(vm, left).ok();
        let right_string = crate::run::structural_ops::copy_string(vm, right).ok();
        if left_string.is_some() || right_string.is_some() {
            match (left_string, right_string) {
                (Some(left), Some(right)) if left == right => continue,
                (Some(_), Some(_)) => return Ok(false),
                _ => return Err(Error::msg("equal-value runtime type mismatch")),
            }
        }
        let left_path = crate::run::structural_ops::copy_path(vm, left).ok();
        let right_path = crate::run::structural_ops::copy_path(vm, right).ok();
        if left_path.is_some() || right_path.is_some() {
            match (left_path, right_path) {
                (Some(left), Some(right)) if left == right => continue,
                (Some(_), Some(_)) => return Ok(false),
                _ => return Err(Error::msg("equal-value runtime type mismatch")),
            }
        }
        let left_structural = crate::run::structural_ops::semantic_snapshot(vm, left).ok();
        let right_structural = crate::run::structural_ops::semantic_snapshot(vm, right).ok();
        if left_structural.is_some() || right_structural.is_some() {
            return match (left_structural, right_structural) {
                (Some(left), Some(right)) => Ok(left == right),
                _ => Err(Error::msg("equal-value runtime type mismatch")),
            };
        }
        match (vm.arena.get(left)?, vm.arena.get(right)?) {
            (
                HeapObj::Enum {
                    layout: left_layout,
                    physical_tag: left_tag,
                    ..
                },
                HeapObj::Enum {
                    layout: right_layout,
                    physical_tag: right_tag,
                    ..
                },
            ) if left_layout == right_layout && left_tag != right_tag => return Ok(false),
            (
                HeapObj::Enum {
                    layout: left_layout,
                    physical_tag: left_tag,
                    active_payload: left_payload,
                },
                HeapObj::Enum {
                    layout: right_layout,
                    physical_tag: right_tag,
                    active_payload: right_payload,
                },
            ) if left_layout == right_layout
                && left_tag == right_tag
                && left_payload.len() == right_payload.len() =>
            {
                pending.extend(
                    left_payload
                        .iter()
                        .copied()
                        .zip(right_payload.iter().copied()),
                );
            }
            _ => return Err(Error::msg("equal-value runtime type mismatch")),
        }
    }
    Ok(true)
}

fn symbol_text(chunk: &lkjscript_core::ValidatedChunk, symbol: u32) -> Result<&str> {
    match chunk.constants().get(symbol as usize) {
        Some(lkjscript_core::Constant::Symbol(text)) => Ok(text),
        _ => Err(Error::msg("invalid symbol constant index")),
    }
}

pub(crate) fn equal_value<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let equal = value_equal(vm, left, right)?;
    vm.push(Value::from_bool(equal));
    Ok(())
}

pub(crate) fn same_object<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let equal = match (left.as_resource(), right.as_resource()) {
        (Some(left), Some(right)) => left == right,
        (Some(_), None) | (None, Some(_)) => {
            return Err(Error::msg("same-object runtime type mismatch"));
        }
        (None, None) => return Err(Error::msg("same-object expects Resource")),
    };
    vm.push(Value::from_bool(equal));
    Ok(())
}

include!("equality/list.rs");

pub(crate) fn f64_bits_equal<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let right = right
        .as_f64_bits()
        .ok_or_else(|| Error::msg("f64-bits-equal expects F64"))?;
    let left = left
        .as_f64_bits()
        .ok_or_else(|| Error::msg("f64-bits-equal expects F64"))?;
    vm.push(Value::from_bool(left == right));
    Ok(())
}
