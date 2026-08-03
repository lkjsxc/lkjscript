use super::*;

impl JitStructuralRuntime {
    pub(super) fn convert_numeric(
        &mut self,
        input: NativeValue,
        kind: StructuralNumericConversion,
        success: &StructuralAggregateDescriptor,
        failure: &StructuralAggregateDescriptor,
        errors: &[StructuralAggregateDescriptor],
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        let converted = match (kind, input) {
            (StructuralNumericConversion::F64FromI64Exact, NativeValue::I64(value)) => {
                lkjscript_core::f64_from_i64_exact(value)
                    .map(|value| NativeValue::F64Bits(value.to_bits()))
            }
            (StructuralNumericConversion::I64FromF64Exact, NativeValue::F64Bits(bits)) => {
                lkjscript_core::i64_from_f64_exact(f64::from_bits(bits)).map(NativeValue::I64)
            }
            (StructuralNumericConversion::I64FromF64Truncating, NativeValue::F64Bits(bits)) => {
                lkjscript_core::i64_from_f64_trunc(f64::from_bits(bits)).map(NativeValue::I64)
            }
            _ => return Err(NativeServiceError::Trap),
        };
        match converted {
            Ok(value) => self.finish_numeric_variant(success, Some(value)),
            Err(error) => {
                let descriptor = numeric_error_descriptor(errors, error)?;
                let error_owner = self.finish_numeric_variant(descriptor, None)?;
                self.finish_numeric_variant(
                    failure,
                    Some(NativeValue::StructuralOwner(error_owner)),
                )
            }
        }
    }

    fn finish_numeric_variant(
        &mut self,
        aggregate: &StructuralAggregateDescriptor,
        payload: Option<NativeValue>,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        let storage = StructuralStorageRoute::Unique;
        let destination = self.create_destination(aggregate, storage)?;
        let destination = match payload {
            Some(value) => {
                self.initialize_destination(destination, value, aggregate, storage, 0)?
            }
            None if aggregate.fields().is_empty() => destination,
            None => return Err(NativeServiceError::HostFailure),
        };
        self.finish_destination(destination, aggregate, storage)
    }
}

fn numeric_error_descriptor(
    errors: &[StructuralAggregateDescriptor],
    error: NumericError,
) -> Result<&StructuralAggregateDescriptor, NativeServiceError> {
    errors
        .iter()
        .find(|descriptor| {
            matches!(
                descriptor.kind(),
                StructuralAggregateKind::Enum(tag) if tag == error.physical_tag()
            )
        })
        .ok_or(NativeServiceError::HostFailure)
}
