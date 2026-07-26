use super::enum_metadata::{enum_facts, operation_ids};
use super::*;
use crate::*;

impl JitHeapServices<'_> {
    pub(crate) fn execute_enums(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        match site.descriptor().operation() {
            operation @ HeapOperation::EnumValue { substitutions, .. } => {
                let facts = enum_facts(self.enums, operation).map_err(|message| {
                    self.last_trap = Some(message.into());
                    NativeServiceError::Trap
                })?;
                if substitutions.len() > 16 || arguments.len() != facts.field_count {
                    return self.trap("enum construction substitution/field metadata mismatch");
                }
                if self.logical_aggregate_constructions >= self.max_logical_aggregate_constructions
                {
                    self.last_resource = Some(ResourceLimitKind::LogicalAggregateConstructions);
                    return Err(NativeServiceError::ResourceLimitExceeded);
                }
                self.logical_aggregate_constructions += 1;
                self.preflight_enum_heap(arguments)?;
                let mut payload = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    payload.push(self.value_from_native(*argument)?);
                }
                let reference_type = match site.descriptor().result_type() {
                    ValueType::Reference(reference_type @ ReferenceType::Enum(_)) => reference_type,
                    _ => return self.trap("enum construction result layout mismatch"),
                };
                let value = self.allocate(
                    HeapObj::Enum {
                        layout: facts.layout,
                        physical_tag: facts.physical_tag,
                        active_payload: payload,
                    },
                    reference_type,
                )?;
                self.native_from_value(value, site.descriptor().result_type())
            }
            operation @ HeapOperation::EnumIsVariant { .. } => {
                let facts = enum_facts(self.enums, operation).map_err(|message| {
                    self.last_trap = Some(message.into());
                    NativeServiceError::Trap
                })?;
                let value = self.enum_argument(arguments)?;
                let (layout, tag) = match self.heap.get(value) {
                    Ok(HeapObj::Enum {
                        layout,
                        physical_tag,
                        ..
                    }) => (*layout, *physical_tag),
                    _ => return self.trap("enum variant test expects enum"),
                };
                self.validate_runtime_enum(layout, tag, operation)?;
                Ok(NativeValue::Bool(tag == facts.physical_tag))
            }
            operation @ HeapOperation::EnumField { .. } => {
                let facts = enum_facts(self.enums, operation).map_err(|message| {
                    self.last_trap = Some(message.into());
                    NativeServiceError::Trap
                })?;
                let value = self.enum_argument(arguments)?;
                let projected = match self.heap.get(value) {
                    Ok(HeapObj::Enum {
                        layout,
                        physical_tag,
                        active_payload,
                    }) if *layout == facts.layout && *physical_tag == facts.physical_tag => facts
                        .field_index
                        .and_then(|index| active_payload.get(index))
                        .copied(),
                    Ok(HeapObj::Enum { .. }) => {
                        return self.trap("inactive enum projection rejected before payload access")
                    }
                    _ => return self.trap("enum projection expects enum"),
                }
                .ok_or_else(|| {
                    self.last_trap = Some("enum active payload is malformed".into());
                    NativeServiceError::Trap
                })?;
                self.native_from_value(projected, site.descriptor().result_type())
            }
            _ => Err(NativeServiceError::HostFailure),
        }
    }

    fn preflight_enum_heap(&mut self, arguments: &[NativeValue]) -> Result<(), NativeServiceError> {
        let scalar_boxes = arguments
            .iter()
            .filter(|value| match value {
                NativeValue::F64Bits(_) => true,
                NativeValue::I64(value) => Value::from_small_i64(*value).is_none(),
                _ => false,
            })
            .count();
        self.heap
            .preflight_enum_allocations(scalar_boxes, arguments.len())
            .map_err(|limit| {
                self.last_resource = Some(match limit {
                    GcLimit::Allocations => ResourceLimitKind::Allocations,
                    GcLimit::HeapBytes => ResourceLimitKind::HeapBytes,
                });
                NativeServiceError::ResourceLimitExceeded
            })
    }

    fn enum_argument(&mut self, arguments: &[NativeValue]) -> Result<Value, NativeServiceError> {
        match arguments {
            [NativeValue::Reference(reference)] => native_reference_value(self.heap, *reference)
                .map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                }),
            _ => self.trap("enum runtime argument metadata mismatch"),
        }
    }

    fn validate_runtime_enum(
        &mut self,
        layout: lkjscript_core::RuntimeLayoutId,
        physical_tag: u16,
        operation: &HeapOperation,
    ) -> Result<(), NativeServiceError> {
        let valid = operation_ids(operation)
            .and_then(|(enum_id, _, layout_id)| {
                self.enums.iter().find(|definition| {
                    definition.id.bytes() == enum_id
                        && definition.layout.identity.bytes() == layout_id
                })
            })
            .is_some_and(|definition| {
                layout.bytes() == definition.layout.identity.bytes()
                    && definition
                        .variants
                        .iter()
                        .any(|variant| variant.physical_tag == physical_tag)
            });
        if valid {
            Ok(())
        } else {
            self.trap("enum runtime layout/physical tag is malformed")
        }
    }
}
