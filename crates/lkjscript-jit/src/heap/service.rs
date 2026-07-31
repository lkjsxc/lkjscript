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
            .map_err(|limit| match limit {
                GcLimit::Allocations => {
                    self.last_resource = Some(ResourceLimitKind::Allocations);
                    NativeServiceError::ResourceLimitExceeded
                }
                GcLimit::HeapBytes => {
                    self.last_resource = Some(ResourceLimitKind::HeapBytes);
                    NativeServiceError::ResourceLimitExceeded
                }
                GcLimit::MixedOwnershipGraph => {
                    self.last_trap = Some(
                        "legacy traced object cannot contain deterministic owners or capabilities"
                            .into(),
                    );
                    NativeServiceError::Trap
                }
            })
    }

    pub(crate) fn value_from_native(
        &mut self,
        value: NativeValue,
    ) -> Result<Value, NativeServiceError> {
        match value {
            NativeValue::Unit => Ok(Value::UNIT),
            NativeValue::Bool(value) => Ok(Value::from_bool(value)),
            NativeValue::I64(value) => Ok(Value::from_i64(value)),
            NativeValue::F64Bits(bits) => Ok(Value::from_f64_bits(bits)),
            NativeValue::StaticBytes(_)
            | NativeValue::StaticString(_)
            | NativeValue::Capability(_)
            | NativeValue::Resource(_)
            | NativeValue::Unique(_)
            | NativeValue::Loan(_)
            | NativeValue::StructuralOwner(_)
            | NativeValue::StructuralView(_)
            | NativeValue::StructuralDestination(_) => {
                self.trap("island value entered legacy heap service")
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
            ValueType::I64 => value.as_i64().map(NativeValue::I64).ok_or_else(|| {
                self.last_trap = Some("heap operation produced non-I64".into());
                NativeServiceError::Trap
            }),
            ValueType::F64 => value
                .as_f64_bits()
                .map(NativeValue::F64Bits)
                .ok_or_else(|| {
                    self.last_trap = Some("heap operation produced non-F64".into());
                    NativeServiceError::Trap
                }),
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
