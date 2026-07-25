use super::*;
use crate::*;

impl<'a> JitHeapServices<'a> {
    pub(crate) fn new(heap: &'a mut GcHeap, force_collection: bool) -> Self {
        Self {
            heap,
            force_collection,
            last_trap: None,
            last_resource: None,
        }
    }

    pub(crate) fn trap<T>(&mut self, message: impl Into<String>) -> Result<T, NativeServiceError> {
        self.last_trap = Some(message.into());
        Err(NativeServiceError::Trap)
    }

    pub(crate) fn roots(&mut self, roots: &[NativeRoot]) -> Result<Vec<Value>, NativeServiceError> {
        roots
            .iter()
            .map(|root| {
                native_reference_value(
                    self.heap,
                    lkjscript_native::NativeReference::new(
                        root.reference_type(),
                        root.opaque_word(),
                    ),
                )
                .map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })
            })
            .collect()
    }

    pub(crate) fn allocate(
        &mut self,
        object: HeapObj,
        reference_type: ReferenceType,
    ) -> Result<Value, NativeServiceError> {
        self.heap
            .try_alloc_with_layout(object, reference_layout_key(reference_type))
            .map_err(|limit| {
                self.last_resource = Some(match limit {
                    GcLimit::Allocations => ResourceLimitKind::Allocations,
                    GcLimit::HeapBytes => ResourceLimitKind::HeapBytes,
                });
                NativeServiceError::ResourceLimitExceeded
            })
    }

    pub(crate) fn mutate<T>(
        &mut self,
        value: Value,
        mutation: impl FnOnce(&mut HeapObj) -> lkjscript_core::Result<T>,
    ) -> Result<T, NativeServiceError> {
        self.heap.mutate(value, mutation).map_err(|error| {
            if error.class() == ErrorClass::Resource(ResourceLimitKind::HeapBytes) {
                self.last_resource = Some(ResourceLimitKind::HeapBytes);
                NativeServiceError::ResourceLimitExceeded
            } else {
                self.last_trap = Some(error.to_string());
                NativeServiceError::Trap
            }
        })
    }

    pub(crate) fn result_error(
        &mut self,
        message: &str,
        result_type: ValueType,
    ) -> Result<NativeValue, NativeServiceError> {
        let payload = self.allocate(HeapObj::Str(message.into()), ReferenceType::Str)?;
        let reference_type = result_type
            .reference_type()
            .ok_or(NativeServiceError::HostFailure)?;
        let result = self.allocate(HeapObj::ResultErr(payload), reference_type)?;
        self.native_from_value(result, result_type)
    }

    pub(crate) fn value_from_native(
        &mut self,
        value: NativeValue,
    ) -> Result<Value, NativeServiceError> {
        match value {
            NativeValue::Unit => Ok(Value::UNIT),
            NativeValue::Bool(value) => Ok(Value::from_bool(value)),
            NativeValue::I64(value) => match Value::from_small_i64(value) {
                Some(value) => Ok(value),
                None => self.allocate(HeapObj::Int(value), scalar_box_layout(1)),
            },
            NativeValue::F64Bits(bits) => {
                self.allocate(HeapObj::Float(f64::from_bits(bits)), scalar_box_layout(2))
            }
            NativeValue::Reference(reference) => native_reference_value(self.heap, reference)
                .map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                }),
        }
    }

    pub(crate) fn native_from_value(
        &mut self,
        value: Value,
        expected: ValueType,
    ) -> Result<NativeValue, NativeServiceError> {
        match expected {
            ValueType::Unit if value.is_unit() => Ok(NativeValue::Unit),
            ValueType::Bool => value.as_bool().map(NativeValue::Bool).ok_or_else(|| {
                self.last_trap = Some("heap operation produced non-Bool".into());
                NativeServiceError::Trap
            }),
            ValueType::I64 => {
                let number = if let Some(number) = value.as_small_i64() {
                    Some(number)
                } else {
                    match self.heap.get(value) {
                        Ok(HeapObj::Int(number)) => Some(*number),
                        _ => None,
                    }
                };
                number.map(NativeValue::I64).ok_or_else(|| {
                    self.last_trap = Some("heap operation produced non-I64".into());
                    NativeServiceError::Trap
                })
            }
            ValueType::F64 => match self.heap.get(value) {
                Ok(HeapObj::Float(number)) => Ok(NativeValue::F64Bits(number.to_bits())),
                _ => self.trap("heap operation produced non-F64"),
            },
            ValueType::Reference(reference_type) => {
                reference_native_value(self.heap, value, reference_type).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })
            }
            _ => self.trap("heap operation result category mismatch"),
        }
    }
}

impl JitHeapServices<'_> {
    pub(crate) fn execute(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        match site.descriptor().operation() {
            HeapOperation::ConstantStr(_)
            | HeapOperation::EmptyStr
            | HeapOperation::EmptyList
            | HeapOperation::None
            | HeapOperation::ProductValue { .. }
            | HeapOperation::ProductField { .. }
            | HeapOperation::WithProductField { .. } => self.execute_products(site, arguments),
            HeapOperation::Cons
            | HeapOperation::Car
            | HeapOperation::Cdr
            | HeapOperation::IsEmptyList
            | HeapOperation::Some
            | HeapOperation::Ok
            | HeapOperation::Err
            | HeapOperation::IsSome
            | HeapOperation::UnwrapSome
            | HeapOperation::IsOk
            | HeapOperation::UnwrapOk
            | HeapOperation::UnwrapErr => self.execute_lists(site, arguments),
            HeapOperation::BufNew
            | HeapOperation::BufLen
            | HeapOperation::BufRef
            | HeapOperation::BufGetU32
            | HeapOperation::BufSet
            | HeapOperation::BufSetU32 => self.execute_buffer_access(site, arguments),
            HeapOperation::BufClone
            | HeapOperation::BufFromStr
            | HeapOperation::BufToStr
            | HeapOperation::BufSlice => self.execute_buffer_transfer(site, arguments),
            HeapOperation::StrLen
            | HeapOperation::StrRef
            | HeapOperation::StrAppend
            | HeapOperation::StrSlice
            | HeapOperation::StrFromByte
            | HeapOperation::StrFromI64
            | HeapOperation::StrFromF64 => self.execute_strings(site, arguments),
            HeapOperation::EqualValue | HeapOperation::SameObject | HeapOperation::ListEqual => {
                self.execute_equality(site, arguments)
            }
        }
    }
}
