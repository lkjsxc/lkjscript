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
            HeapOperation::Some | HeapOperation::Ok | HeapOperation::Err => {
                let inner = self.value_from_native(argument(0)?)?;
                let object = match descriptor.operation() {
                    HeapOperation::Some => HeapObj::OptionSome(inner),
                    HeapOperation::Ok => HeapObj::ResultOk(inner),
                    HeapOperation::Err => HeapObj::ResultErr(inner),
                    _ => return Err(NativeServiceError::HostFailure),
                };
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(object, reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::IsSome => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                Ok(NativeValue::Bool(!value.is_none()))
            }
            HeapOperation::UnwrapSome => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                if value.is_none() {
                    return self.trap("unwrap-some on none");
                }
                let inner = match self.heap.get(value) {
                    Ok(HeapObj::OptionSome(inner)) => *inner,
                    _ => return self.trap("unwrap-some operand is not Option"),
                };
                self.native_from_value(inner, result_type)
            }
            HeapOperation::IsOk => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                match self.heap.get(value) {
                    Ok(HeapObj::ResultOk(_)) => Ok(NativeValue::Bool(true)),
                    Ok(HeapObj::ResultErr(_)) => Ok(NativeValue::Bool(false)),
                    _ => self.trap("is-ok operand is not Result"),
                }
            }
            HeapOperation::UnwrapOk | HeapOperation::UnwrapErr => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let inner = match (descriptor.operation(), self.heap.get(value)) {
                    (HeapOperation::UnwrapOk, Ok(HeapObj::ResultOk(inner)))
                    | (HeapOperation::UnwrapErr, Ok(HeapObj::ResultErr(inner))) => *inner,
                    (HeapOperation::UnwrapOk, Ok(HeapObj::ResultErr(error))) => {
                        let message = match self.heap.get(*error) {
                            Ok(HeapObj::Str(message)) => format!("unwrap-ok: {message}"),
                            _ => "unwrap-ok on Err".to_string(),
                        };
                        return self.trap(message);
                    }
                    (HeapOperation::UnwrapErr, Ok(HeapObj::ResultOk(_))) => {
                        return self.trap("unwrap-err on Ok")
                    }
                    _ => return self.trap("unwrap Result category mismatch"),
                };
                self.native_from_value(inner, result_type)
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }
}
