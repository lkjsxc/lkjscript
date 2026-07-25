use super::*;

fn maybe_i64(arena: &lkjscript_core::GcHeap, value: Value) -> Result<Option<i64>> {
    if let Some(number) = value.as_small_i64() {
        return Ok(Some(number));
    }
    let Some(_) = value.as_heap() else {
        return Ok(None);
    };
    match arena.get(value)? {
        HeapObj::Int(number) => Ok(Some(*number)),
        _ => Ok(None),
    }
}

fn value_equal(arena: &lkjscript_core::GcHeap, mut left: Value, mut right: Value) -> Result<bool> {
    loop {
        if left.is_unit() || right.is_unit() {
            return if left.is_unit() && right.is_unit() {
                Ok(true)
            } else {
                Err(Error::msg("equal-value runtime type mismatch"))
            };
        }

        let left_bool = left.as_bool();
        let right_bool = right.as_bool();
        if left_bool.is_some() || right_bool.is_some() {
            return match (left_bool, right_bool) {
                (Some(left), Some(right)) => Ok(left == right),
                _ => Err(Error::msg("equal-value runtime type mismatch")),
            };
        }

        let left_i64 = maybe_i64(arena, left)?;
        let right_i64 = maybe_i64(arena, right)?;
        if left_i64.is_some() || right_i64.is_some() {
            return match (left_i64, right_i64) {
                (Some(left), Some(right)) => Ok(left == right),
                _ => Err(Error::msg("equal-value runtime type mismatch")),
            };
        }

        if left.is_none() || right.is_none() {
            if left.is_none() && right.is_none() {
                return Ok(true);
            }
            let other = if left.is_none() { right } else { left };
            return match arena.get(other)? {
                HeapObj::OptionSome(_) => Ok(false),
                _ => Err(Error::msg("equal-value runtime type mismatch")),
            };
        }

        let (next_left, next_right) = match (arena.get(left)?, arena.get(right)?) {
            (HeapObj::Float(left), HeapObj::Float(right)) => return Ok(left == right),
            (HeapObj::Str(left), HeapObj::Str(right)) => return Ok(left == right),
            (HeapObj::Symbol(left), HeapObj::Symbol(right)) => return Ok(left == right),
            (HeapObj::OptionSome(left), HeapObj::OptionSome(right))
            | (HeapObj::ResultOk(left), HeapObj::ResultOk(right))
            | (HeapObj::ResultErr(left), HeapObj::ResultErr(right)) => (*left, *right),
            (HeapObj::ResultOk(_), HeapObj::ResultErr(_))
            | (HeapObj::ResultErr(_), HeapObj::ResultOk(_)) => return Ok(false),
            _ => return Err(Error::msg("equal-value runtime type mismatch")),
        };
        left = next_left;
        right = next_right;
    }
}

pub(crate) fn equal_value<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let equal = value_equal(&vm.arena, left, right)?;
    vm.push(Value::from_bool(equal));
    Ok(())
}

pub(crate) fn same_object<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let equal = match (left.as_handle(), right.as_handle()) {
        (Some(left), Some(right)) => left == right,
        (Some(_), None) | (None, Some(_)) => {
            return Err(Error::msg("same-object runtime type mismatch"));
        }
        (None, None) => match (vm.arena.get(left)?, vm.arena.get(right)?) {
            (HeapObj::Buf(_), HeapObj::Buf(_)) => left.raw() == right.raw(),
            _ => return Err(Error::msg("same-object expects Buf or Handle")),
        },
    };
    vm.push(Value::from_bool(equal));
    Ok(())
}

fn list_node(arena: &lkjscript_core::GcHeap, value: Value) -> Result<Option<(Value, Value)>> {
    if value.is_empty_list() {
        return Ok(None);
    }
    match arena.get(value)? {
        HeapObj::Pair { car, cdr } => Ok(Some((*car, *cdr))),
        _ => Err(Error::msg("list-equal expects proper List values")),
    }
}

pub(crate) fn list_values_equal(
    arena: &lkjscript_core::GcHeap,
    mut left: Value,
    mut right: Value,
    limit: usize,
) -> Result<bool> {
    let mut steps = 0_usize;
    loop {
        let left_node = list_node(arena, left)?;
        let right_node = list_node(arena, right)?;
        let (left_car, left_cdr, right_car, right_cdr) = match (left_node, right_node) {
            (None, None) => return Ok(true),
            (None, Some(_)) | (Some(_), None) => return Ok(false),
            (Some((left_car, left_cdr)), Some((right_car, right_cdr))) => {
                (left_car, left_cdr, right_car, right_cdr)
            }
        };
        if steps == limit {
            return Err(Error::msg("list-equal step limit exceeded"));
        }
        steps += 1;
        if !value_equal(arena, left_car, right_car)? {
            return Ok(false);
        }
        left = left_cdr;
        right = right_cdr;
    }
}

pub(crate) fn list_equal<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let equal = list_values_equal(&vm.arena, left, right, MAX_LIST_EQUAL_STEPS)?;
    vm.push(Value::from_bool(equal));
    Ok(())
}

pub(crate) fn f64_bits_equal<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let right = match vm.arena.get(right)? {
        HeapObj::Float(number) => number.to_bits(),
        _ => return Err(Error::msg("f64-bits-equal expects F64")),
    };
    let left = match vm.arena.get(left)? {
        HeapObj::Float(number) => number.to_bits(),
        _ => return Err(Error::msg("f64-bits-equal expects F64")),
    };
    vm.push(Value::from_bool(left == right));
    Ok(())
}
