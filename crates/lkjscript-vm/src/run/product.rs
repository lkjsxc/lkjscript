use super::*;

#[path = "structural_ops/aggregate/product_metadata.rs"]
mod metadata;
#[path = "structural_ops/aggregate/product_region.rs"]
mod region;
use metadata::*;
use region::*;

fn make_product<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let product = ProductId::new(vm.read_u16()?);
    let (field_count, identity, routes) = {
        let metadata = product_metadata(vm, product)?;
        if !metadata.region {
            return Err(Error::msg(
                "product construction requires invocation-region metadata",
            ));
        }
        (
            metadata.fields.len(),
            metadata.identity,
            metadata.region_fields.clone(),
        )
    };
    let mut fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        fields.push(vm.pop()?);
    }
    fields.reverse();
    for (route, value) in routes.iter().copied().zip(fields.iter().copied()) {
        if !region_field_value(vm, route, value)? {
            return Err(Error::msg(
                "region-product construction field route mismatch",
            ));
        }
    }
    charge_region_product(vm, fields.capacity())?;
    let key = vm
        .region_products
        .as_mut()
        .ok_or_else(|| Error::msg("region-product arena is unavailable"))?
        .publish(identity, fields)
        .map_err(region_product_error)?;
    vm.region_product_allocations = vm.region_product_allocations.saturating_add(1);
    vm.push(Value::from_region_product(key));
    Ok(())
}

fn load_product_field<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let descriptor = vm.read_u16()? as usize;
    let field_ref = product_field_ref(vm, descriptor)?;
    let identity = {
        let metadata = product_metadata(vm, field_ref.product)?;
        if !metadata.region {
            return Err(Error::msg(
                "product projection requires invocation-region metadata",
            ));
        }
        metadata.identity
    };
    let value = vm.pop()?;
    let key = region_product_key(vm, value)?;
    let field = *vm
        .region_products
        .as_ref()
        .ok_or_else(|| Error::msg("region-product arena is unavailable"))?
        .field(key, identity, u16::from(field_ref.field))
        .map_err(region_product_error)?;
    vm.push(field);
    Ok(())
}

fn with_product_field<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let descriptor = vm.read_u16()? as usize;
    let field_ref = product_field_ref(vm, descriptor)?;
    let (identity, route) = {
        let metadata = product_metadata(vm, field_ref.product)?;
        if !metadata.region {
            return Err(Error::msg(
                "product update requires invocation-region metadata",
            ));
        }
        (
            metadata.identity,
            metadata
                .region_fields
                .get(usize::from(field_ref.field))
                .copied(),
        )
    };
    let replacement = vm.pop()?;
    let route = route.ok_or_else(|| Error::msg("region-product field route is missing"))?;
    if !region_field_value(vm, route, replacement)? {
        return Err(Error::msg(
            "region-product replacement field route mismatch",
        ));
    }
    let value = vm.pop()?;
    let key = region_product_key(vm, value)?;
    let field_count = vm
        .region_products
        .as_ref()
        .ok_or_else(|| Error::msg("region-product arena is unavailable"))?
        .fields(key, identity)
        .map_err(region_product_error)?
        .len();
    charge_region_product(vm, field_count)?;
    let updated = vm
        .region_products
        .as_mut()
        .ok_or_else(|| Error::msg("region-product arena is unavailable"))?
        .update(key, identity, u16::from(field_ref.field), replacement)
        .map_err(region_product_error)?;
    vm.region_product_allocations = vm.region_product_allocations.saturating_add(1);
    vm.push(Value::from_region_product(updated));
    Ok(())
}

use lkjscript_core::Op;

pub(super) fn handles(op: u8) -> bool {
    op == Op::MakeProduct as u8
        || op == Op::LoadProductField as u8
        || op == Op::WithProductField as u8
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match op {
        x if x == Op::MakeProduct as u8 => make_product(vm),
        x if x == Op::LoadProductField as u8 => load_product_field(vm),
        x if x == Op::WithProductField as u8 => with_product_field(vm),
        _ => unreachable!("opcode family checked"),
    }
}
