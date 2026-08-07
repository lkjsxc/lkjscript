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
        let allocations = self
            .list_allocations
            .checked_add(self.region_product_allocations)
            .ok_or(NativeServiceError::HostFailure)?;
        if self
            .max_list_allocations
            .is_some_and(|maximum| allocations >= maximum)
        {
            self.last_resource = Some(ResourceLimitKind::Allocations);
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        let increase = self
            .region_products
            .publish_storage_increase(fields)
            .map_err(|_| NativeServiceError::HostFailure)?;
        let list_bytes = self
            .lists
            .reserved_bytes_estimate()
            .map_err(|_| NativeServiceError::HostFailure)?;
        let region_bytes = self
            .region_products
            .reserved_bytes_estimate()
            .map_err(|_| NativeServiceError::HostFailure)?;
        let projected = list_bytes
            .checked_add(region_bytes)
            .and_then(|bytes| bytes.checked_add(increase))
            .ok_or(NativeServiceError::HostFailure)?;
        if self
            .max_runtime_bytes
            .is_some_and(|maximum| projected > maximum)
        {
            self.last_resource = Some(ResourceLimitKind::HeapBytes);
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        Ok(())
    }

    pub(super) fn region_product_error(
        &mut self,
        error: lkjscript_core::RegionProductError,
    ) -> NativeServiceError {
        match error {
            lkjscript_core::RegionProductError::HostAllocation
            | lkjscript_core::RegionProductError::ArithmeticOverflow
            | lkjscript_core::RegionProductError::RepresentationExhausted => {
                NativeServiceError::HostFailure
            }
            _ => {
                self.last_trap = Some(format!("region-product operation failed: {error:?}"));
                NativeServiceError::Trap
            }
        }
    }
}
