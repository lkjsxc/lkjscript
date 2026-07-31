use super::*;

impl JitValueServices<'_> {
    pub(super) fn region_key(
        &mut self,
        reference: lkjscript_native::NativeReference,
    ) -> Result<lkjscript_core::RegionProductKey, NativeServiceError> {
        lkjscript_core::RegionProductKey::from_word(
            self.region_products.id(),
            reference.opaque_word(),
        )
        .ok_or_else(|| {
            self.last_trap = Some("invalid region-product reference".into());
            NativeServiceError::Trap
        })
    }

    pub(super) fn charge_region_product(
        &mut self,
        fields: usize,
    ) -> Result<(), NativeServiceError> {
        if self.logical_aggregate_constructions >= self.max_logical_aggregate_constructions {
            self.last_resource = Some(ResourceLimitKind::Allocations);
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        let allocations = self
            .list_allocations
            .saturating_add(self.region_product_allocations);
        if allocations >= self.max_list_allocations {
            self.last_resource = Some(ResourceLimitKind::Allocations);
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        let increase = self.region_products.publish_storage_increase(fields);
        let projected = self
            .lists
            .reserved_bytes_estimate()
            .saturating_add(self.region_products.metrics().reserved_bytes_estimate)
            .saturating_add(increase);
        if projected > self.max_runtime_bytes {
            self.last_resource = Some(ResourceLimitKind::HeapBytes);
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        self.logical_aggregate_constructions =
            self.logical_aggregate_constructions.saturating_add(1);
        Ok(())
    }

    pub(super) fn region_product_error(
        &mut self,
        error: lkjscript_core::RegionProductError,
    ) -> NativeServiceError {
        match error {
            lkjscript_core::RegionProductError::Records
            | lkjscript_core::RegionProductError::Fields
            | lkjscript_core::RegionProductError::HostAllocation => {
                self.last_resource = Some(ResourceLimitKind::Allocations);
                NativeServiceError::ResourceLimitExceeded
            }
            _ => {
                self.last_trap = Some(format!("region-product operation failed: {error:?}"));
                NativeServiceError::Trap
            }
        }
    }
}
