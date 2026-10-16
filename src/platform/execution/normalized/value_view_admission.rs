//! Bounded negative guards for ordinary value boundaries. These checks mint no
//! value/category/provenance certificate and never acquire a read-view lease.

use super::super::value_schema::NormalizedValueSchema;
use super::{MAXIMUM_ADMISSION_ITEMS, NormalizedRecord, NormalizedValue};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{TypeForm, TypeObjectDigest};

struct Guard<'a> {
    control: &'a ExecutionControl,
    visits: u64,
    ancestors: [Option<TypeObjectDigest>; 257],
}

impl Guard<'_> {
    // Traversal borrows existing storage and allocates no worklist or child buffer.
    // The existing raw-value depth/item bounds also bound the host recursion here.
    fn enter(&mut self, depth: u16) -> Result<(), ExecutionError> {
        self.control.check()?;
        if depth > 256 {
            return Err(ExecutionError::resource(
                "normalized_value_depth",
                "view escape inspection exceeds finite value depth 256",
            ));
        }
        self.visits = self
            .visits
            .checked_add(1)
            .filter(|visits| *visits <= MAXIMUM_ADMISSION_ITEMS + 1)
            .ok_or_else(|| {
                ExecutionError::resource(
                    "normalized_collection_items",
                    "view escape inspection exceeds finite admission items",
                )
            })?;
        Ok(())
    }

    fn value(&mut self, value: &NormalizedValue, depth: u16) -> Result<(), ExecutionError> {
        self.enter(depth)?;
        match value {
            NormalizedValue::ReadView(_) => return Err(escape()),
            NormalizedValue::Record(NormalizedRecord::Nominal { fields, .. }) => {
                for child in fields.iter() {
                    self.value(child, depth + 1)?;
                }
            }
            NormalizedValue::Record(NormalizedRecord::Structural { fields }) => {
                for (_, child) in fields.iter() {
                    self.value(child, depth + 1)?;
                }
            }
            NormalizedValue::List(values) => {
                for child in values.iter() {
                    self.value(child, depth + 1)?;
                }
            }
            NormalizedValue::Map(values) => {
                for (_, child) in values.iter() {
                    self.value(child, depth + 1)?;
                }
            }
            NormalizedValue::Variant { payload, .. } | NormalizedValue::Option(payload) => {
                if let Some(child) = payload.as_deref() {
                    self.value(child, depth + 1)?;
                }
            }
            NormalizedValue::Result { value, .. } => self.value(value, depth + 1)?,
            NormalizedValue::Function {
                read_scope_arguments,
                bound_arguments,
                ..
            } => {
                // Ordinary first-class callable types have no scope/result-relation
                // carrier. Retaining supplied scope actuals cannot grant that authority.
                if !read_scope_arguments.is_empty() {
                    return Err(escape());
                }
                if let Some(children) = bound_arguments {
                    for child in children.iter() {
                        self.value(child, depth + 1)?;
                    }
                }
            }
            // Opaque owner/resource storage is admitted independently by its consuming
            // owner; this negative guard does not certify any hidden payload as ordinary.
            NormalizedValue::OwnedSequence(_)
            | NormalizedValue::OwnedChoice(_)
            | NormalizedValue::OwnedProduct(_)
            | NormalizedValue::OwnedI64Cell(_)
            | NormalizedValue::ByteBuffer(_)
            | NormalizedValue::Resource(_)
            | NormalizedValue::Unit
            | NormalizedValue::Bool(_)
            | NormalizedValue::I64(_)
            | NormalizedValue::F64(_)
            | NormalizedValue::Bytes(_)
            | NormalizedValue::Text(_)
            | NormalizedValue::StaticText(_) => {}
        }
        Ok(())
    }

    fn ty<S: NormalizedValueSchema + ?Sized>(
        &mut self,
        schema: &S,
        ty: TypeObjectDigest,
        depth: u16,
    ) -> Result<(), ExecutionError> {
        self.enter(depth)?;
        // Ordinary recursive nominal data remains legal. An exact ancestor is
        // already inspecting all of its outgoing edges, including inactive cases.
        if self.ancestors[..usize::from(depth)].contains(&Some(ty)) {
            return Ok(());
        }
        self.ancestors[usize::from(depth)] = Some(ty);
        let object = schema.types().get(&ty).ok_or_else(|| {
            ExecutionError::new(
                crate::platform::execution::ExecutionFailureClass::Trap,
                "normalized_value_admission",
                "view escape inspection requires every exact type in the selected schema",
            )
        })?;
        match &object.form {
            TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => return Err(escape()),
            TypeForm::RegionBrand { .. }
            | TypeForm::Region { .. }
            | TypeForm::RelPtr { .. }
            | TypeForm::RegionSlice { .. }
            | TypeForm::RegionCap { .. } => {
                return Err(ExecutionError::new(
                    crate::platform::execution::ExecutionFailureClass::Trap,
                    "normalized_region_escape",
                    "a region-bound type cannot cross an unscoped value boundary",
                ));
            }
            TypeForm::Named { .. } | TypeForm::Applied { .. } => {
                // Phantom actuals and inactive variant payloads are still occurrences.
                if let TypeForm::Applied { arguments, .. } = &object.form {
                    for child in arguments {
                        self.ty(schema, *child, depth + 1)?;
                    }
                }
                if let Some(index) = schema.record_index(ty) {
                    let record = schema.records().get(index).ok_or_else(layout_error)?;
                    for child in record.arguments.iter() {
                        self.ty(schema, *child, depth + 1)?;
                    }
                    for field in record.fields.iter() {
                        self.ty(schema, field.ty, depth + 1)?;
                    }
                }
                if let Some(index) = schema.variant_index(ty) {
                    let variant = schema.variants().get(index).ok_or_else(layout_error)?;
                    for child in variant.arguments.iter() {
                        self.ty(schema, *child, depth + 1)?;
                    }
                    for case in variant.cases.iter() {
                        if let Some(child) = case.payload {
                            self.ty(schema, child, depth + 1)?;
                        }
                    }
                }
            }
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                for field in fields {
                    self.ty(schema, field.ty, depth + 1)?;
                }
            }
            TypeForm::List { item }
            | TypeForm::Option { item }
            | TypeForm::Stream { item }
            | TypeForm::OwnedSequence { item } => self.ty(schema, *item, depth + 1)?,
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                self.ty(schema, *key, depth + 1)?;
                self.ty(schema, *value, depth + 1)?;
            }
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => {
                for child in parameters {
                    self.ty(schema, *child, depth + 1)?;
                }
                self.ty(schema, *result, depth + 1)?;
            }
            // Abstract non-view forms grant no boundary category certificate. Their
            // substitution and exact consuming-context admission remain separate.
            TypeForm::TypeParameter { .. }
            | TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. }
            | TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText
            | TypeForm::Secret
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::CapabilityResource { .. } => {}
        }
        Ok(())
    }
}

pub(super) fn reject_read_view_value_escape(
    value: &NormalizedValue,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    Guard {
        control,
        visits: 0,
        ancestors: [None; 257],
    }
    .value(value, 0)
}

pub(in super::super) fn reject_read_view_type_escape<S: NormalizedValueSchema + ?Sized>(
    schema: &S,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    Guard {
        control,
        visits: 0,
        ancestors: [None; 257],
    }
    .ty(schema, ty, 0)
}

fn escape() -> ExecutionError {
    ExecutionError::new(
        crate::platform::execution::ExecutionFailureClass::Trap,
        "normalized_read_view_escape",
        "read-view values, scopes, or nested types cannot cross an ordinary value boundary",
    )
}

fn layout_error() -> ExecutionError {
    ExecutionError::new(
        crate::platform::execution::ExecutionFailureClass::Trap,
        "normalized_value_admission",
        "view escape inspection found an absent nominal layout",
    )
}
