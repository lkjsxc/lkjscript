//! Bounded test-only structural oracle. It deliberately ignores both evaluator classifications.

use super::reference_schema::NormalizedReferenceSchema;
use super::value::{NormalizedMapKey, NormalizedRecord, NormalizedValue, ValueOrigin};
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{TypeForm, TypeObjectDigest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Affinity {
    Free,
    Handle,
    Variant,
}

pub(super) fn inspect(
    schema: &NormalizedReferenceSchema,
    origin: ValueOrigin,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
) -> Result<(Affinity, u64), &'static str> {
    fn structural_class(
        schema: &NormalizedReferenceSchema,
        origin: ValueOrigin,
        value: &NormalizedValue,
    ) -> Result<Affinity, &'static str> {
        match value {
            NormalizedValue::Resource(handle) if handle.is_affine_capability() => {
                Ok(Affinity::Handle)
            }
            NormalizedValue::Variant { layout, .. } => {
                if layout.1 != origin {
                    return Err("foreign preparation");
                }
                let variant = schema
                    .variants
                    .get(layout.0 as usize)
                    .ok_or("foreign variant layout")?;
                // This slow case scan is intentional and must never be used by execution.
                for case in variant.cases.iter() {
                    if let Some(ty) = case.payload
                        && matches!(
                            schema.types.get(&ty).map(|ty| &ty.form),
                            Some(TypeForm::CapabilityResource { .. })
                        )
                    {
                        return Ok(Affinity::Variant);
                    }
                }
                Ok(Affinity::Free)
            }
            _ => Ok(Affinity::Free),
        }
    }
    let affinity = structural_class(schema, origin, value)?;
    let mut stack = vec![(value, ty, 0_u16, true)];
    let mut nodes = 0_u64;
    let mut items = 0_usize;
    while let Some((value, ty, depth, owner)) = stack.pop() {
        control.check().map_err(|_| "cancelled")?;
        nodes = nodes.checked_add(1).ok_or("oracle node overflow")?;
        if depth > 256 {
            return Err("oracle depth");
        }
        let class = structural_class(schema, origin, value)?;
        if !owner && class != Affinity::Free {
            return Err("nested affine owner");
        }
        let form = &schema.types.get(&ty).ok_or("foreign exact type")?.form;
        let count = match value {
            NormalizedValue::List(values)
            | NormalizedValue::Record(NormalizedRecord::Nominal { fields: values, .. }) => {
                values.len()
            }
            NormalizedValue::Record(NormalizedRecord::Structural { fields }) => fields.len(),
            NormalizedValue::Map(values) => values.len(),
            NormalizedValue::Variant { payload, .. } | NormalizedValue::Option(payload) => {
                usize::from(payload.is_some())
            }
            NormalizedValue::Result { .. } => 1,
            _ => 0,
        };
        items = items
            .checked_add(count)
            .filter(|count| *count <= 1_000_000)
            .ok_or("oracle items")?;
        let mut children = Vec::with_capacity(count);
        match (value, form) {
            (NormalizedValue::Unit, TypeForm::Unit)
            | (NormalizedValue::Bool(_), TypeForm::Bool)
            | (NormalizedValue::I64(_), TypeForm::I64)
            | (NormalizedValue::Bytes(_), TypeForm::Bytes)
            | (NormalizedValue::Text(_), TypeForm::Text)
            | (NormalizedValue::StaticText(_), TypeForm::StaticText) => {}
            (
                NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }),
                TypeForm::Named { declaration },
            ) => {
                if layout.1 != origin {
                    return Err("foreign preparation");
                }
                let record = schema
                    .records
                    .get(layout.0 as usize)
                    .filter(|record| {
                        record.declaration == *declaration && record.fields.len() == fields.len()
                    })
                    .ok_or("foreign nominal shape")?;
                for (child, field) in fields.iter().zip(record.fields.iter()) {
                    children.push((child, field.ty, false));
                }
            }
            (
                NormalizedValue::Record(NormalizedRecord::Structural { fields }),
                TypeForm::StructuralRecord { fields: expected },
            ) => {
                if fields.len() != expected.len() {
                    return Err("foreign structural arity");
                }
                for ((name, child), field) in fields.iter().zip(expected) {
                    if *name != field.name {
                        return Err("foreign structural field");
                    }
                    children.push((child, field.ty, false));
                }
            }
            (
                NormalizedValue::Variant {
                    layout,
                    case,
                    payload,
                },
                TypeForm::Named { declaration },
            ) => {
                let variant = schema
                    .variants
                    .get(layout.0 as usize)
                    .filter(|variant| variant.declaration == *declaration)
                    .ok_or("foreign variant identity")?;
                let selected = variant
                    .cases
                    .get(*case as usize)
                    .ok_or("foreign variant tag")?;
                match (payload.as_deref(), selected.payload) {
                    (None, None) => {}
                    (Some(value), Some(ty)) => children.push((
                        value,
                        ty,
                        owner
                            && class == Affinity::Variant
                            && matches!(
                                schema.types.get(&ty).map(|ty| &ty.form),
                                Some(TypeForm::CapabilityResource { .. })
                            ),
                    )),
                    _ => return Err("foreign payload presence"),
                }
            }
            (NormalizedValue::List(values), TypeForm::List { item }) => {
                for value in values.iter() {
                    children.push((value, *item, false));
                }
            }
            (NormalizedValue::Option(value), TypeForm::Option { item }) => {
                if let Some(value) = value.as_deref() {
                    children.push((value, *item, false));
                }
            }
            (NormalizedValue::Result { success, value }, TypeForm::Result { ok, error }) => {
                children.push((value.as_ref(), if *success { *ok } else { *error }, false));
            }
            (
                NormalizedValue::Map(values),
                TypeForm::Map {
                    key: expected,
                    value: item,
                },
            ) => {
                let expected = &schema.types.get(expected).ok_or("foreign map type")?.form;
                for (key, value) in values.iter() {
                    if !matches!(
                        (key, expected),
                        (NormalizedMapKey::I64(_), TypeForm::I64)
                            | (NormalizedMapKey::Bool(_), TypeForm::Bool)
                            | (NormalizedMapKey::Bytes(_), TypeForm::Bytes)
                            | (
                                NormalizedMapKey::Text(_),
                                TypeForm::Text | TypeForm::StaticText
                            )
                    ) {
                        return Err("foreign map key type");
                    }
                    children.push((value, *item, false));
                }
            }
            (NormalizedValue::Resource(handle), TypeForm::CapabilityResource { .. })
                if handle.is_affine_capability() => {}
            (NormalizedValue::Resource(handle), TypeForm::Stream { .. })
                if !handle.is_affine_capability() => {}
            _ => return Err("foreign value shape"),
        }
        stack.extend(
            children
                .into_iter()
                .rev()
                .map(|(value, ty, owner)| (value, ty, depth + 1, owner)),
        );
    }
    Ok((affinity, nodes))
}

std::thread_local! {
    static FORCED_RESCAN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(super) fn force_rescan<T>(run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            FORCED_RESCAN.set(self.0);
        }
    }
    let _reset = Reset(FORCED_RESCAN.replace(true));
    run()
}

/// Restore payload work safely in tests without changing the computed classification.
pub(super) fn forced_descendant_work(value: &NormalizedValue, work: &mut super::value::ValueWork) {
    if !FORCED_RESCAN.get() {
        return;
    }
    let mut pending = vec![value];
    let mut nodes = 0_u64;
    while let Some(value) = pending.pop() {
        nodes = nodes.saturating_add(1);
        if nodes > 1_000_001 {
            break;
        }
        match value {
            NormalizedValue::Record(NormalizedRecord::Nominal { fields, .. })
            | NormalizedValue::List(fields) => pending.extend(fields.iter()),
            NormalizedValue::Record(NormalizedRecord::Structural { fields }) => {
                pending.extend(fields.iter().map(|(_, value)| value))
            }
            NormalizedValue::Variant { payload, .. } | NormalizedValue::Option(payload) => {
                pending.extend(payload.as_deref())
            }
            NormalizedValue::Map(entries) => pending.extend(entries.values()),
            NormalizedValue::Result { value, .. } => pending.push(value),
            NormalizedValue::Function {
                bound_arguments: Some(prefix),
                ..
            } => pending.extend(prefix.iter()),
            _ => {}
        }
    }
    work.internal_guard_descendant_visits = work
        .internal_guard_descendant_visits
        .saturating_add(nodes.saturating_sub(1));
}
