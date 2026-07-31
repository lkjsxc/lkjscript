use super::*;
use crate::*;

impl JitHeapServices<'_> {
    pub(crate) fn execute_equality(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let descriptor = site.descriptor();
        let _result_type = descriptor.result_type();
        let argument = |index: usize| {
            arguments
                .get(index)
                .copied()
                .ok_or(NativeServiceError::HostFailure)
        };
        let _as_i64 = |value: NativeValue| match value {
            NativeValue::I64(value) => Ok(value),
            _ => Err(NativeServiceError::HostFailure),
        };
        let _as_f64 = |value: NativeValue| match value {
            NativeValue::F64Bits(bits) => Ok(f64::from_bits(bits)),
            _ => Err(NativeServiceError::HostFailure),
        };
        let as_reference = |value: NativeValue| match value {
            NativeValue::Reference(reference) => Ok(reference),
            _ => Err(NativeServiceError::HostFailure),
        };
        match descriptor.operation() {
            HeapOperation::EqualValue => {
                let left = self.value_from_native(argument(0)?)?;
                let right = self.value_from_native(argument(1)?)?;
                let equal = value_equal(self.heap, left, right).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })?;
                Ok(NativeValue::Bool(equal))
            }
            HeapOperation::ListEqual => {
                let left = native_reference_value(self.heap, as_reference(argument(0)?)?).map_err(
                    |message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    },
                )?;
                let right = native_reference_value(self.heap, as_reference(argument(1)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let equal = list_values_equal(self.heap, left, right, MAX_LIST_EQUAL_STEPS)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                Ok(NativeValue::Bool(equal))
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }
}
