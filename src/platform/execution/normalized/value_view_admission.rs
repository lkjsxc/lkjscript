//! Negative guards for ordinary value boundaries. The value graph is traversed
//! iteratively without a callable-environment depth ceiling. Structural type
//! traversal retains its separate existing depth contract. These checks mint no
//! value/category/provenance certificate and never acquire a read-view lease.

use super::super::value_schema::NormalizedValueSchema;
use super::{NormalizedRecord, NormalizedValue};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{TypeForm, TypeObjectDigest};

struct TypeGuard<'a> {
    control: &'a ExecutionControl,
    ancestors: [Option<TypeObjectDigest>; 257],
}

impl TypeGuard<'_> {
    // Traversal borrows existing storage and allocates no worklist or child buffer.
    // The existing nesting bound limits host recursion. Width follows finite
    // child iteration rather than an admission-work counter.
    fn enter(&mut self, depth: u16) -> Result<(), ExecutionError> {
        self.control.check()?;
        if depth > 256 {
            return Err(ExecutionError::resource(
                "normalized_value_depth",
                "view escape inspection exceeds finite value depth 256",
            ));
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

/// A unary Function/Record chain uses only the inline current reference.
/// A branching frontier reserves borrowed references fallibly. No opaque owner
/// is cloned, no value is published, and every logical occurrence is visited.
struct ValuePending<'a> {
    current: Option<&'a NormalizedValue>,
    others: Vec<&'a NormalizedValue>,
}

impl<'a> ValuePending<'a> {
    fn push(&mut self, value: &'a NormalizedValue) -> Result<(), ExecutionError> {
        if let Some(previous) = self.current {
            self.others.try_reserve(1).map_err(|_| {
                ExecutionError::resource_static(
                    "normalized_value_escape_allocation",
                    "value escape inspection could not reserve its pending references",
                )
            })?;
            self.others.push(previous);
        }
        self.current = Some(value);
        Ok(())
    }

    fn pop(&mut self) -> Option<&'a NormalizedValue> {
        self.current.take().or_else(|| self.others.pop())
    }
}

pub(super) fn reject_read_view_value_escape(
    value: &NormalizedValue,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    let mut pending = ValuePending {
        current: Some(value),
        others: Vec::new(),
    };
    while let Some(value) = pending.pop() {
        control.check()?;
        match value {
            NormalizedValue::ReadView(_)
            | NormalizedValue::RegionCap { .. }
            | NormalizedValue::RegionPointer { .. } => return Err(escape()),
            NormalizedValue::Record(NormalizedRecord::Nominal { fields, .. }) => {
                for child in fields.iter().rev() {
                    control.check()?;
                    pending.push(child)?;
                }
            }
            NormalizedValue::Record(NormalizedRecord::Structural { fields }) => {
                for (_, child) in fields.iter().rev() {
                    control.check()?;
                    pending.push(child)?;
                }
            }
            NormalizedValue::List(values) => {
                for child in values.iter().rev() {
                    control.check()?;
                    pending.push(child)?;
                }
            }
            NormalizedValue::Map(values) => {
                for (_, child) in values.iter().rev() {
                    control.check()?;
                    pending.push(child)?;
                }
            }
            NormalizedValue::Variant { payload, .. } | NormalizedValue::Option(payload) => {
                if let Some(child) = payload.as_deref() {
                    control.check()?;
                    pending.push(child)?;
                }
            }
            NormalizedValue::Result { value, .. } => {
                control.check()?;
                pending.push(value)?;
            }
            NormalizedValue::Function {
                read_scope_arguments,
                bound_arguments,
                ..
            } => {
                if !read_scope_arguments.is_empty() {
                    return Err(escape());
                }
                if let Some(children) = bound_arguments {
                    for child in children.iter().rev() {
                        control.check()?;
                        pending.push(child)?;
                    }
                }
            }
            // Hidden owner/resource payloads remain the responsibility of their
            // consuming owner. This guard grants no ordinary-value certificate.
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
    }
    Ok(())
}

pub(in super::super) fn reject_read_view_type_escape<S: NormalizedValueSchema + ?Sized>(
    schema: &S,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    TypeGuard {
        control,
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
