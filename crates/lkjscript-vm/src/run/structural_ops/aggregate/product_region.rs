use super::*;

pub(super) fn region_product_key<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    value: Value,
) -> Result<lkjscript_core::RegionProductKey> {
    let word = value
        .as_region_product_word()
        .ok_or_else(|| Error::msg("region-product operation expects a region product"))?;
    let arena = vm
        .region_products
        .as_ref()
        .ok_or_else(|| Error::msg("region-product arena is unavailable"))?;
    lkjscript_core::RegionProductKey::from_word(arena.id(), word)
        .ok_or_else(|| Error::msg("region-product key is stale or malformed"))
}

pub(super) fn region_field_value<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    route: lkjscript_core::RegionProductFieldKind,
    value: Value,
) -> Result<bool> {
    Ok(match route {
        lkjscript_core::RegionProductFieldKind::Unit => value.is_unit(),
        lkjscript_core::RegionProductFieldKind::Bool => value.as_bool().is_some(),
        lkjscript_core::RegionProductFieldKind::I64 => value.as_i64().is_some(),
        lkjscript_core::RegionProductFieldKind::F64 => value.as_f64_bits().is_some(),
        lkjscript_core::RegionProductFieldKind::List => {
            value.is_empty_list() || value.as_segmented_list().is_some()
        }
        lkjscript_core::RegionProductFieldKind::Product(product) => {
            let key = region_product_key(vm, value)?;
            let identity = product_metadata(vm, product)?.identity;
            vm.region_products
                .as_ref()
                .ok_or_else(|| Error::msg("region-product arena is unavailable"))?
                .validate_identity(key, identity)
                .map_err(region_product_error)?;
            true
        }
    })
}

pub(super) fn charge_region_product<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    fields: usize,
) -> Result<()> {
    vm.preflight_allocation(1)?;
    let region = vm
        .region_products
        .as_ref()
        .ok_or_else(|| Error::msg("region-product arena is unavailable"))?;
    vm.preflight_heap_growth(
        region
            .publish_storage_increase(fields)
            .map_err(region_product_error)?,
    )
}

pub(super) fn region_product_error(error: lkjscript_core::RegionProductError) -> Error {
    match error {
        lkjscript_core::RegionProductError::HostAllocation
        | lkjscript_core::RegionProductError::ArithmeticOverflow
        | lkjscript_core::RegionProductError::RepresentationExhausted => Error::host(format!(
            "region-product representation or host allocation failed: {error:?}"
        )),
        _ => Error::msg(format!("region-product operation failed: {error:?}")),
    }
}
