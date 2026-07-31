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
    if vm.logical_aggregate_constructions >= vm.config.max_logical_aggregate_constructions {
        return Err(Error::resource(
            ResourceLimitKind::Allocations,
            "VM region-product logical construction limit exceeded",
        ));
    }
    let allocations = vm
        .list_allocations
        .saturating_add(vm.region_product_allocations);
    if allocations >= vm.config.max_allocations {
        return Err(Error::resource(
            ResourceLimitKind::Allocations,
            "VM region-product allocation limit exceeded",
        ));
    }
    let region = vm
        .region_products
        .as_ref()
        .ok_or_else(|| Error::msg("region-product arena is unavailable"))?;
    let projected = vm
        .list_reserved_bytes_estimate()
        .saturating_add(region.metrics().reserved_bytes_estimate)
        .saturating_add(region.publish_storage_increase(fields));
    if projected > u64::try_from(vm.config.max_heap_bytes).unwrap_or(u64::MAX) {
        return Err(Error::resource(
            ResourceLimitKind::HeapBytes,
            "VM region-product heap-byte limit exceeded",
        ));
    }
    vm.logical_aggregate_constructions = vm.logical_aggregate_constructions.saturating_add(1);
    Ok(())
}

pub(super) fn region_product_error(error: lkjscript_core::RegionProductError) -> Error {
    match error {
        lkjscript_core::RegionProductError::Records
        | lkjscript_core::RegionProductError::Fields
        | lkjscript_core::RegionProductError::HostAllocation => Error::resource(
            ResourceLimitKind::Allocations,
            format!("region-product allocation failed: {error:?}"),
        ),
        _ => Error::msg(format!("region-product operation failed: {error:?}")),
    }
}
