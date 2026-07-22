//! Opcode dispatch.

use lkjscript_core::{
    Error, HeapObj, JitHook, Op, ProductFieldRef, ProductId, Result, Value, MAX_LIST_EQUAL_STEPS,
    MAX_PRODUCT_FIELDS,
};

use super::calls::{call, car, cdr, make_closure};
use super::numeric::{bin_arithmetic, bin_ordering, Arithmetic, Ordering};
use super::Vm;
use crate::host::{flush_out, print_value, read_byte, write_byte, write_str};

fn maybe_i64(arena: &crate::arena::Arena, value: Value) -> Result<Option<i64>> {
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

fn value_equal(arena: &crate::arena::Arena, mut left: Value, mut right: Value) -> Result<bool> {
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

fn equal_value<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let equal = value_equal(&vm.arena, left, right)?;
    vm.push(Value::from_bool(equal));
    Ok(())
}

fn same_object<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
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

fn list_node(arena: &crate::arena::Arena, value: Value) -> Result<Option<(Value, Value)>> {
    if value.is_empty_list() {
        return Ok(None);
    }
    match arena.get(value)? {
        HeapObj::Pair { car, cdr } => Ok(Some((*car, *cdr))),
        _ => Err(Error::msg("list-equal expects proper List values")),
    }
}

fn list_values_equal(
    arena: &crate::arena::Arena,
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

fn list_equal<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let equal = list_values_equal(&vm.arena, left, right, MAX_LIST_EQUAL_STEPS)?;
    vm.push(Value::from_bool(equal));
    Ok(())
}

fn f64_bits_equal<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
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

fn product_metadata<'a, J: JitHook>(
    vm: &'a Vm<'_, J>,
    product: ProductId,
) -> Result<&'a lkjscript_core::ProductMetadata> {
    let metadata = vm
        .chunk
        .products
        .get(product.index())
        .filter(|metadata| metadata.id == product)
        .ok_or_else(|| Error::msg("product metadata index or identity is invalid"))?;
    if metadata.fields.len() > MAX_PRODUCT_FIELDS {
        return Err(Error::msg("product metadata exceeds field limit"));
    }
    Ok(metadata)
}

fn product_field_ref<J: JitHook>(vm: &Vm<'_, J>, index: usize) -> Result<ProductFieldRef> {
    let field_ref = vm
        .chunk
        .product_fields
        .get(index)
        .copied()
        .ok_or_else(|| Error::msg("product field descriptor index out of range"))?;
    let metadata = product_metadata(vm, field_ref.product)?;
    if usize::from(field_ref.field) >= metadata.fields.len() {
        return Err(Error::msg("product field index out of range"));
    }
    Ok(field_ref)
}

fn make_product<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let product = ProductId::new(vm.read_u16()?);
    let field_count = product_metadata(vm, product)?.fields.len();
    let mut fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        fields.push(vm.pop()?);
    }
    fields.reverse();
    let value = vm.arena.alloc(HeapObj::Product { product, fields });
    vm.push(value);
    Ok(())
}

fn load_product_field<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let descriptor = vm.read_u16()? as usize;
    let field_ref = product_field_ref(vm, descriptor)?;
    let value = vm.pop()?;
    if value.as_heap().is_none() {
        return Err(Error::msg("product field access expects Product"));
    }
    let field = match vm.arena.get(value)? {
        HeapObj::Product { product, fields } if *product == field_ref.product => fields
            .get(usize::from(field_ref.field))
            .copied()
            .ok_or_else(|| Error::msg("product value field count does not match metadata"))?,
        HeapObj::Product { .. } => {
            return Err(Error::msg("product field access identity mismatch"));
        }
        _ => return Err(Error::msg("product field access expects Product")),
    };
    vm.push(field);
    Ok(())
}

fn with_product_field<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let descriptor = vm.read_u16()? as usize;
    let field_ref = product_field_ref(vm, descriptor)?;
    let replacement = vm.pop()?;
    let value = vm.pop()?;
    if value.as_heap().is_none() {
        return Err(Error::msg("product field replacement expects Product"));
    }
    let mut fields = match vm.arena.get(value)? {
        HeapObj::Product { product, fields } if *product == field_ref.product => fields.clone(),
        HeapObj::Product { .. } => {
            return Err(Error::msg("product field replacement identity mismatch"));
        }
        _ => return Err(Error::msg("product field replacement expects Product")),
    };
    let field = fields
        .get_mut(usize::from(field_ref.field))
        .ok_or_else(|| Error::msg("product value field count does not match metadata"))?;
    *field = replacement;
    let updated = vm.arena.alloc(HeapObj::Product {
        product: field_ref.product,
        fields,
    });
    vm.push(updated);
    Ok(())
}

fn bin_bits<J: JitHook>(vm: &mut Vm<'_, J>, f: fn(i64, i64) -> i64) -> Result<()> {
    let right = vm.pop()?;
    let left = vm.pop()?;
    let right = vm
        .as_i64(right)
        .map_err(|_| Error::msg("bit op expects I64"))?;
    let left = vm
        .as_i64(left)
        .map_err(|_| Error::msg("bit op expects I64"))?;
    let result = vm.make_i64(f(left, right));
    vm.push(result);
    Ok(())
}

pub fn dispatch<J: JitHook>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match op {
        x if x == Op::Nop as u8 => Ok(()),
        x if x == Op::LoadConst as u8 => {
            let id = vm.read_u16()? as usize;
            let v = vm.load_const(id)?;
            vm.push(v);
            Ok(())
        }
        x if x == Op::LoadLocal as u8 => {
            let slot = vm.read_u8()? as usize;
            let base = vm
                .frames
                .last()
                .ok_or_else(|| Error::msg("LoadLocal without frame"))?
                .locals_base;
            let value = vm
                .stack
                .get(base + slot)
                .copied()
                .ok_or_else(|| Error::msg("LoadLocal slot out of range"))?;
            if value.is_invalid() {
                return Err(Error::msg("LoadLocal read uninitialized slot"));
            }
            vm.push(value);
            Ok(())
        }
        x if x == Op::StoreLocal as u8 => {
            let slot = vm.read_u8()? as usize;
            let base = vm
                .frames
                .last()
                .ok_or_else(|| Error::msg("StoreLocal without frame"))?
                .locals_base;
            let value = vm.peek()?;
            let target = vm
                .stack
                .get_mut(base + slot)
                .ok_or_else(|| Error::msg("StoreLocal slot out of range"))?;
            *target = value;
            Ok(())
        }
        x if x == Op::LoadGlobal as u8 => {
            let id = vm.read_u16()? as usize;
            let value = vm
                .globals
                .get(id)
                .copied()
                .ok_or_else(|| Error::msg("LoadGlobal slot out of range"))?;
            if value.is_invalid() {
                return Err(Error::msg("LoadGlobal read uninitialized slot"));
            }
            vm.push(value);
            Ok(())
        }
        x if x == Op::StoreGlobal as u8 => {
            let id = vm.read_u16()? as usize;
            let value = vm.peek()?;
            let target = vm
                .globals
                .get_mut(id)
                .ok_or_else(|| Error::msg("StoreGlobal slot out of range"))?;
            *target = value;
            Ok(())
        }
        x if x == Op::Add as u8 => bin_arithmetic(vm, Arithmetic::Add),
        x if x == Op::Sub as u8 => bin_arithmetic(vm, Arithmetic::Subtract),
        x if x == Op::Mul as u8 => bin_arithmetic(vm, Arithmetic::Multiply),
        x if x == Op::Div as u8 => bin_arithmetic(vm, Arithmetic::Divide),
        x if x == Op::EqualValue as u8 => equal_value(vm),
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
        x if x == Op::Jump as u8 => {
            let at = vm.read_u16()? as usize;
            if let Some(fr) = vm.frames.last_mut() {
                fr.ip = at;
            }
            Ok(())
        }
        x if x == Op::JumpIfFalse as u8 => {
            let at = vm.read_u16()? as usize;
            let condition = vm
                .pop()?
                .as_bool()
                .ok_or_else(|| Error::msg("JumpIfFalse expects Bool"))?;
            if !condition {
                let frame = vm
                    .frames
                    .last_mut()
                    .ok_or_else(|| Error::msg("JumpIfFalse without frame"))?;
                frame.ip = at;
            }
            Ok(())
        }
        x if x == Op::MakeClosure as u8 => make_closure(vm),
        x if x == Op::MakeProduct as u8 => make_product(vm),
        x if x == Op::LoadProductField as u8 => load_product_field(vm),
        x if x == Op::WithProductField as u8 => with_product_field(vm),
        x if x == Op::Call as u8 => {
            let argc = vm.read_u8()?;
            call(vm, argc)
        }
        x if x == Op::Return as u8 => {
            let ret = vm.pop()?;
            let frame = vm.frames.pop().ok_or_else(|| Error::msg("return"))?;
            vm.stack.truncate(frame.stack_base);
            vm.push(ret);
            Ok(())
        }
        x if x == Op::Cons as u8 => {
            let cdr_v = vm.pop()?;
            let car_v = vm.pop()?;
            let v = vm.arena.alloc(HeapObj::Pair {
                car: car_v,
                cdr: cdr_v,
            });
            vm.push(v);
            Ok(())
        }
        x if x == Op::Car as u8 => car(vm),
        x if x == Op::Cdr as u8 => cdr(vm),
        x if x == Op::IsEmptyList as u8 => {
            let v = vm.pop()?;
            vm.push(Value::from_bool(v.is_empty_list()));
            Ok(())
        }
        x if x == Op::SameObject as u8 => same_object(vm),
        x if x == Op::ListEqual as u8 => list_equal(vm),
        x if x == Op::F64BitsEqual as u8 => f64_bits_equal(vm),
        x if x == Op::Print as u8 => {
            let v = vm.pop()?;
            print_value(&vm.arena, v)?;
            vm.push(Value::UNIT);
            Ok(())
        }
        x if x == Op::Flush as u8 => {
            flush_out()?;
            vm.push(Value::UNIT);
            Ok(())
        }
        x if x == Op::ReadByte as u8 => {
            let number = read_byte()?;
            let value = vm.make_i64(number);
            vm.push(value);
            Ok(())
        }
        x if x == Op::WriteByte as u8 => {
            let value = vm.pop()?;
            let byte = vm.as_i64(value)?;
            vm.push(write_byte(byte)?);
            Ok(())
        }
        x if x == Op::WriteStr as u8 => {
            let v = vm.pop()?;
            vm.push(write_str(&vm.arena, v)?);
            Ok(())
        }
        x if x == Op::Exit as u8 => {
            let value = vm.pop()?;
            let code = vm.as_i64(value)?;
            let code = i32::try_from(code).map_err(|_| Error::msg("exit code out of range"))?;
            vm.exit_code = Some(code);
            Ok(())
        }
        x if crate::run::ext_ops::dispatch_ext(vm, x)? => Ok(()),
        x if x == Op::Pop as u8 => {
            let _ = vm.pop()?;
            Ok(())
        }
        x if x == Op::Dup as u8 => {
            let v = vm.peek()?;
            vm.push(v);
            Ok(())
        }
        x if x == Op::False as u8 => {
            vm.push(Value::FALSE);
            Ok(())
        }
        x if x == Op::True as u8 => {
            vm.push(Value::TRUE);
            Ok(())
        }
        x if x == Op::Unit as u8 => {
            vm.push(Value::UNIT);
            Ok(())
        }
        x if x == Op::EmptyList as u8 => {
            vm.push(Value::EMPTY_LIST);
            Ok(())
        }
        x if x == Op::OptionNone as u8 => {
            vm.push(Value::NONE);
            Ok(())
        }
        other => Err(Error::msg(format!("unknown opcode {other}"))),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use lkjscript_core::{Chunk, HeapObj, NullJit, Op, Value};

    use super::{dispatch, list_values_equal};
    use crate::run::Vm;

    fn compare(vm: &mut Vm<'_, NullJit>, op: Op, left: Value, right: Value) -> bool {
        vm.push(left);
        vm.push(right);
        dispatch(vm, op as u8).expect("comparison succeeds");
        vm.pop()
            .expect("comparison result")
            .as_bool()
            .expect("Bool result")
    }

    fn i64_list(vm: &mut Vm<'_, NullJit>, values: &[i64]) -> Value {
        let mut list = Value::EMPTY_LIST;
        for number in values.iter().rev() {
            let car = vm.make_i64(*number);
            list = vm.arena.alloc(HeapObj::Pair { car, cdr: list });
        }
        list
    }

    #[test]
    fn value_equality_is_exact_and_category_checked() {
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());

        assert!(compare(&mut vm, Op::EqualValue, Value::UNIT, Value::UNIT));
        assert!(!compare(&mut vm, Op::EqualValue, Value::TRUE, Value::FALSE));
        let wide_left = vm.make_i64(i64::MAX);
        let wide_right = vm.make_i64(i64::MAX);
        assert!(compare(&mut vm, Op::EqualValue, wide_left, wide_right));

        let positive_zero = vm.arena.alloc(HeapObj::Float(0.0));
        let negative_zero = vm.arena.alloc(HeapObj::Float(-0.0));
        assert!(compare(
            &mut vm,
            Op::EqualValue,
            positive_zero,
            negative_zero
        ));
        let nan_left = vm.arena.alloc(HeapObj::Float(f64::NAN));
        let nan_right = vm.arena.alloc(HeapObj::Float(f64::NAN));
        assert!(!compare(&mut vm, Op::EqualValue, nan_left, nan_right));

        let text_left = vm.arena.alloc(HeapObj::Str("same".into()));
        let text_right = vm.arena.alloc(HeapObj::Str("same".into()));
        assert!(compare(&mut vm, Op::EqualValue, text_left, text_right));
        let symbol_left = vm.arena.alloc(HeapObj::Symbol("same".into()));
        let symbol_right = vm.arena.alloc(HeapObj::Symbol("same".into()));
        assert!(compare(&mut vm, Op::EqualValue, symbol_left, symbol_right));
        vm.push(text_left);
        vm.push(symbol_left);
        assert!(dispatch(&mut vm, Op::EqualValue as u8).is_err());
    }

    #[test]
    fn option_and_result_value_equality_is_structural() {
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());
        assert!(compare(&mut vm, Op::EqualValue, Value::NONE, Value::NONE));

        let one_left = vm.make_i64(1);
        let one_right = vm.make_i64(1);
        let some_left = vm.arena.alloc(HeapObj::OptionSome(one_left));
        let some_right = vm.arena.alloc(HeapObj::OptionSome(one_right));
        assert!(compare(&mut vm, Op::EqualValue, some_left, some_right));
        assert!(!compare(&mut vm, Op::EqualValue, Value::NONE, some_left));

        let ok_left = vm.arena.alloc(HeapObj::ResultOk(one_left));
        let ok_right = vm.arena.alloc(HeapObj::ResultOk(one_right));
        let err = vm.arena.alloc(HeapObj::ResultErr(one_right));
        assert!(compare(&mut vm, Op::EqualValue, ok_left, ok_right));
        assert!(!compare(&mut vm, Op::EqualValue, ok_left, err));

        let mut deep_left = one_left;
        let mut deep_right = one_right;
        for _ in 0..10_000 {
            deep_left = vm.arena.alloc(HeapObj::OptionSome(deep_left));
            deep_right = vm.arena.alloc(HeapObj::OptionSome(deep_right));
        }
        assert!(compare(&mut vm, Op::EqualValue, deep_left, deep_right));

        let mut result_left = one_left;
        let mut result_right = one_right;
        for _ in 0..10_000 {
            result_left = vm.arena.alloc(HeapObj::ResultOk(result_left));
            result_right = vm.arena.alloc(HeapObj::ResultOk(result_right));
        }
        assert!(compare(&mut vm, Op::EqualValue, result_left, result_right));
    }

    #[test]
    fn object_identity_is_limited_to_buffers_and_handles() {
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());
        let buffer = vm.arena.alloc(HeapObj::Buf(vec![1, 2, 3]));
        let clone = vm.arena.alloc(HeapObj::Buf(vec![1, 2, 3]));
        assert!(compare(&mut vm, Op::SameObject, buffer, buffer));
        assert!(!compare(&mut vm, Op::SameObject, buffer, clone));
        assert!(compare(
            &mut vm,
            Op::SameObject,
            Value::from_handle(7),
            Value::from_handle(7)
        ));
        assert!(!compare(
            &mut vm,
            Op::SameObject,
            Value::from_handle(7),
            Value::from_handle(8)
        ));

        let integer = vm.make_i64(1);
        vm.push(integer);
        vm.push(integer);
        assert!(dispatch(&mut vm, Op::SameObject as u8).is_err());

        let closure = vm.arena.alloc(HeapObj::Closure {
            proto: 0,
            captures: Vec::new(),
        });
        vm.push(closure);
        vm.push(closure);
        assert!(dispatch(&mut vm, Op::EqualValue as u8).is_err());
    }

    #[test]
    fn list_equality_is_structural_bounded_and_rejects_improper_lists() {
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());
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

        let improper_car = vm.make_i64(1);
        let improper_cdr = vm.make_i64(2);
        let improper = vm.arena.alloc(HeapObj::Pair {
            car: improper_car,
            cdr: improper_cdr,
        });
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
    fn f64_bit_equality_distinguishes_signed_zero_and_accepts_equal_nan_bits() {
        let chunk = Chunk::new();
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());
        let positive_zero = vm.arena.alloc(HeapObj::Float(0.0));
        let negative_zero = vm.arena.alloc(HeapObj::Float(-0.0));
        assert!(!compare(
            &mut vm,
            Op::F64BitsEqual,
            positive_zero,
            negative_zero
        ));
        let bits = 0x7ff8_0000_0000_0042_u64;
        let nan_left = vm.arena.alloc(HeapObj::Float(f64::from_bits(bits)));
        let nan_right = vm.arena.alloc(HeapObj::Float(f64::from_bits(bits)));
        assert!(compare(&mut vm, Op::F64BitsEqual, nan_left, nan_right));
        let different_nan = vm
            .arena
            .alloc(HeapObj::Float(f64::from_bits(bits.wrapping_add(1))));
        assert!(!compare(&mut vm, Op::F64BitsEqual, nan_left, different_nan));
    }
}
