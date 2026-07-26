mod results;

use super::*;
use crate::*;

impl<'a> JitHeapServices<'a> {
    pub(crate) fn new(
        heap: &'a mut GcHeap,
        enums: &'a [lkjscript_ir::EnumMetadata],
        force_collection: bool,
        max_logical_aggregate_constructions: u64,
    ) -> Self {
        Self {
            heap,
            enums,
            force_collection,
            logical_aggregate_constructions: 0,
            max_logical_aggregate_constructions,
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

    pub(crate) fn enum_value(
        &mut self,
        layout: [u8; 32],
        physical_tag: u16,
        payload: Vec<Value>,
        reference_type: ReferenceType,
    ) -> Result<Value, NativeServiceError> {
        self.allocate(
            HeapObj::Enum {
                layout: lkjscript_core::RuntimeLayoutId::new(layout),
                physical_tag,
                active_payload: payload,
            },
            reference_type,
        )
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
