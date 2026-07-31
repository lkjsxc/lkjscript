use super::*;
use crate::*;

mod references;

impl<'a> JitValueServices<'a> {
    pub(crate) fn new(
        lists: &'a mut lkjscript_core::SegmentedListArena<Value>,
        region_products: &'a mut lkjscript_core::RegionProductArena<Value>,
        limits: JitValueLimits,
    ) -> Self {
        Self {
            lists,
            region_products,
            logical_aggregate_constructions: 0,
            max_logical_aggregate_constructions: limits.logical_aggregates,
            list_allocations: 0,
            region_product_allocations: 0,
            max_list_allocations: limits.allocations,
            max_runtime_bytes: limits.runtime_bytes,
            last_trap: None,
            last_resource: None,
        }
    }

    pub(crate) fn trap<T>(&mut self, message: impl Into<String>) -> Result<T, NativeServiceError> {
        self.last_trap = Some(message.into());
        Err(NativeServiceError::Trap)
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
                self.trap("island value entered deterministic value service")
            }
            NativeValue::Reference(reference) => {
                self.reference_value(reference).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })
            }
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
            ValueType::Reference(reference_type) => self
                .native_reference(value, reference_type)
                .map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                }),
            _ => self.trap("heap operation result category mismatch"),
        }
    }
}
