use super::*;
use crate::*;

impl JitHeapServices<'_> {
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
            HeapOperation::ProductValue { product, fields } => {
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
                let value = self.allocate(
                    HeapObj::Product {
                        product: ProductId::new(
                            u16::try_from(*product).map_err(|_| NativeServiceError::HostFailure)?,
                        ),
                        fields,
                    },
                    reference_type,
                )?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::ProductField { product, field, .. } => {
                let product_value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let value = match self.heap.get(product_value) {
                    Ok(HeapObj::Product {
                        product: actual,
                        fields,
                    }) if u32::from(actual.raw()) == *product => {
                        fields.get(usize::from(*field)).copied()
                    }
                    _ => return self.trap("product field identity mismatch"),
                }
                .ok_or_else(|| {
                    self.last_trap = Some("product field out of bounds".into());
                    NativeServiceError::Trap
                })?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::WithProductField { product, field, .. } => {
                let source = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let replacement = self.value_from_native(argument(1)?)?;
                let mut fields = match self.heap.get(source) {
                    Ok(HeapObj::Product {
                        product: actual,
                        fields,
                    }) if u32::from(actual.raw()) == *product => fields.clone(),
                    _ => return self.trap("product replacement identity mismatch"),
                };
                let Some(slot) = fields.get_mut(usize::from(*field)) else {
                    return self.trap("product replacement field out of bounds");
                };
                *slot = replacement;
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(
                    HeapObj::Product {
                        product: ProductId::new(
                            u16::try_from(*product).map_err(|_| NativeServiceError::HostFailure)?,
                        ),
                        fields,
                    },
                    reference_type,
                )?;
                self.native_from_value(value, result_type)
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }
}
