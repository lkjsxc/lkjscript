use super::*;
use crate::*;

impl JitHeapServices<'_> {
    pub(crate) fn execute_lists(
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
            HeapOperation::Cons => {
                let car = self.value_from_native(argument(0)?)?;
                let cdr = native_reference_value(self.heap, as_reference(argument(1)?)?).map_err(
                    |message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    },
                )?;
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let pair = self.allocate(HeapObj::Pair { car, cdr }, reference_type)?;
                self.native_from_value(pair, result_type)
            }
            HeapOperation::Car | HeapOperation::Cdr => {
                let list = native_reference_value(self.heap, as_reference(argument(0)?)?).map_err(
                    |message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    },
                )?;
                if list.is_empty_list() {
                    return self.trap(if matches!(descriptor.operation(), HeapOperation::Car) {
                        "car expects pair"
                    } else {
                        "cdr expects pair"
                    });
                }
                let value = match self.heap.get(list) {
                    Ok(HeapObj::Pair { car, cdr }) => {
                        if matches!(descriptor.operation(), HeapOperation::Car) {
                            *car
                        } else {
                            *cdr
                        }
                    }
                    _ => {
                        return self.trap(if matches!(descriptor.operation(), HeapOperation::Car) {
                            "car expects pair"
                        } else {
                            "cdr expects pair"
                        })
                    }
                };
                self.native_from_value(value, result_type)
            }
            HeapOperation::IsEmptyList => {
                let reference = as_reference(argument(0)?)?;
                let value = native_reference_value(self.heap, reference).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })?;
                Ok(NativeValue::Bool(value.is_empty_list()))
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }
}
