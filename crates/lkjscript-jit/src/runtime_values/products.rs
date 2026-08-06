use crate::*;

mod region;

impl JitValueServices<'_> {
    pub(crate) fn execute_products(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let descriptor = site.descriptor();
        let result_type = descriptor.result_type();
        let argument = |index: usize| {
            arguments
                .get(index)
                .copied()
                .ok_or(NativeServiceError::HostFailure)
        };
        let as_reference = |value: NativeValue| match value {
            NativeValue::Reference(reference) => Ok(reference),
            _ => Err(NativeServiceError::HostFailure),
        };
        match descriptor.operation() {
            HeapOperation::EmptyList => Ok(NativeValue::Reference(
                lkjscript_native::NativeReference::new(
                    result_type
                        .reference_type()
                        .ok_or(NativeServiceError::HostFailure)?,
                    0,
                ),
            )),
            HeapOperation::ProductValue { product: _, fields } => {
                if usize::from(*fields) != arguments.len() {
                    return self.trap("product field count mismatch");
                }
                let fields = arguments
                    .iter()
                    .copied()
                    .map(|value| self.value_from_native(value))
                    .collect::<Result<Vec<_>, _>>()?;
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let ReferenceType::RegionProduct(_, identity) = reference_type else {
                    return self.trap("product construction requires invocation-region metadata");
                };
                self.charge_region_product(fields.capacity())?;
                let key = self
                    .region_products
                    .publish(lkjscript_core::RuntimeLayoutId::new(identity), fields)
                    .map_err(|error| self.region_product_error(error))?;
                self.region_product_allocations = self.region_product_allocations.saturating_add(1);
                Ok(NativeValue::Reference(
                    lkjscript_native::NativeReference::new(reference_type, key.to_word()),
                ))
            }
            HeapOperation::ProductField {
                product: _, field, ..
            } => {
                let reference = as_reference(argument(0)?)?;
                let ReferenceType::RegionProduct(_, identity) = reference.reference_type() else {
                    return self.trap("product projection requires invocation-region metadata");
                };
                let key = self.region_key(reference)?;
                let field = u16::from(*field);
                let value = self
                    .region_products
                    .field(
                        key,
                        lkjscript_core::RuntimeLayoutId::new(identity),
                        usize::from(field),
                    )
                    .copied()
                    .map_err(|error| self.region_product_error(error))?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::WithProductField {
                product: _, field, ..
            } => {
                let reference = as_reference(argument(0)?)?;
                let replacement = self.value_from_native(argument(1)?)?;
                let ReferenceType::RegionProduct(_, identity) = reference.reference_type() else {
                    return self.trap("product update requires invocation-region metadata");
                };
                let key = self.region_key(reference)?;
                let field_count = match self
                    .region_products
                    .fields(key, lkjscript_core::RuntimeLayoutId::new(identity))
                {
                    Ok(fields) => fields.len(),
                    Err(error) => return Err(self.region_product_error(error)),
                };
                self.charge_region_product(field_count)?;
                let updated = self
                    .region_products
                    .update(
                        key,
                        lkjscript_core::RuntimeLayoutId::new(identity),
                        usize::from(*field),
                        replacement,
                    )
                    .map_err(|error| self.region_product_error(error))?;
                self.region_product_allocations = self.region_product_allocations.saturating_add(1);
                Ok(NativeValue::Reference(
                    lkjscript_native::NativeReference::new(
                        reference.reference_type(),
                        updated.to_word(),
                    ),
                ))
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }
}
