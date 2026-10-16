//! Fixed cell representation. Both evaluators supply their independently admitted
//! type inventory. This module grants neither type nor capability authority.
use super::value::{NormalizedRecord, NormalizedValue, ValueOrigin};
use crate::platform::execution::{
    ExecutionControl, ExecutionError,
    region::{RegionLayout, runtime},
};
use crate::platform::kernel::{Name, TypeForm, TypeObject, TypeObjectDigest};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) type Arena = runtime::Arena<ValueOrigin>;

pub(super) struct Layout {
    pub(super) storage: RegionLayout,
    size: usize,
    form: Form,
}
enum Form {
    Unit,
    Bool,
    I64,
    Pointer(TypeObjectDigest),
    Record(Vec<(Name, usize, Layout)>),
}

pub(super) fn error() -> ExecutionError {
    ExecutionError::new(
        crate::platform::execution::ExecutionFailureClass::Trap,
        "normalized_region_layout",
        "region value differs from its exact fixed layout or brand",
    )
}

pub(super) fn check_plain(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
    depth: usize,
) -> Result<(), ExecutionError> {
    control.check()?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(error());
    }
    match (&types.get(&ty).ok_or_else(error)?.form, value) {
        (TypeForm::Unit, NormalizedValue::Unit)
        | (TypeForm::Bool, NormalizedValue::Bool(_))
        | (TypeForm::I64, NormalizedValue::I64(_)) => Ok(()),
        (TypeForm::RelPtr { .. }, NormalizedValue::RegionPointer { ty: actual, .. })
            if *actual == ty =>
        {
            Ok(())
        }
        (
            TypeForm::StructuralRecord { fields },
            NormalizedValue::Record(NormalizedRecord::Structural { fields: values }),
        ) if fields.len() == values.len() => {
            for (field, (name, value)) in fields.iter().zip(values.iter()) {
                if field.name != *name {
                    return Err(error());
                }
                check_plain(types, value, field.ty, control, depth + 1)?;
            }
            Ok(())
        }
        _ => Err(error()),
    }
}

impl Layout {
    pub(super) fn derive(
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        ty: TypeObjectDigest,
        region: TypeObjectDigest,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        Self::at(types, ty, region, control, reserve, 0)
    }

    fn at(
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        ty: TypeObjectDigest,
        region: TypeObjectDigest,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
        depth: usize,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(error());
        }
        let mut hasher = blake3::Hasher::new_derive_key("lkjscript.region.cell.v1");
        let (form, size, alignment) = match &types.get(&ty).ok_or_else(error)?.form {
            TypeForm::Unit => {
                hasher.update(&[0]);
                (Form::Unit, 0, 1)
            }
            TypeForm::Bool => {
                hasher.update(&[1]);
                (Form::Bool, 1, 1)
            }
            TypeForm::I64 => {
                hasher.update(&[2]);
                (Form::I64, 8, 8)
            }
            TypeForm::RelPtr {
                region: actual,
                item,
            } if *actual == region => {
                hasher.update(&[3]);
                let mut remaining = super::value::MAXIMUM_ADMISSION_ITEMS;
                fingerprint(
                    types,
                    *item,
                    region,
                    &mut hasher,
                    &mut remaining,
                    control,
                    0,
                )?;
                (Form::Pointer(ty), 4, 4)
            }
            TypeForm::StructuralRecord { fields } => {
                hasher.update(&[4]);
                hasher.update(&(fields.len() as u64).to_le_bytes());
                reserve((fields.len() * std::mem::size_of::<(Name, usize, Layout)>()) as u64)?;
                control.check()?;
                let mut entries = Vec::new();
                entries
                    .try_reserve_exact(fields.len())
                    .map_err(|_| error())?;
                let mut size = 0usize;
                let mut alignment = 1;
                for field in fields {
                    let layout = Self::at(types, field.ty, region, control, reserve, depth + 1)?;
                    let align = layout.storage_alignment();
                    alignment = alignment.max(align);
                    size = align_up(size, align)?;
                    hasher.update(&(field.name.as_str().len() as u64).to_le_bytes());
                    hasher.update(field.name.as_str().as_bytes());
                    hasher.update(&layout.storage_key());
                    reserve(field.name.as_str().len() as u64)?;
                    let end = size.checked_add(layout.size).ok_or_else(error)?;
                    entries.push((field.name.clone(), size, layout));
                    size = end;
                }
                (Form::Record(entries), align_up(size, alignment)?, alignment)
            }
            _ => return Err(error()),
        };
        let storage = RegionLayout::new(
            *hasher.finalize().as_bytes(),
            u32::try_from(size).map_err(|_| error())?,
            alignment as u32,
        )?;
        Ok(Self {
            storage,
            size,
            form,
        })
    }

    fn storage_alignment(&self) -> usize {
        match &self.form {
            Form::Unit | Form::Bool => 1,
            Form::I64 => 8,
            Form::Pointer(_) => 4,
            Form::Record(fields) => fields
                .iter()
                .map(|(_, _, layout)| layout.storage_alignment())
                .max()
                .unwrap_or(1),
        }
    }
    fn storage_key(&self) -> [u8; 32] {
        self.storage.key()
    }

    pub(super) fn encode(
        &self,
        value: &NormalizedValue,
        identity: runtime::Identity,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Vec<u8>, ExecutionError> {
        reserve(self.size as u64)?;
        control.check()?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(self.size).map_err(|_| error())?;
        bytes.resize(self.size, 0);
        self.write(value, identity, &mut bytes, control)?;
        Ok(bytes)
    }

    fn write(
        &self,
        value: &NormalizedValue,
        identity: runtime::Identity,
        bytes: &mut [u8],
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        match (&self.form, value) {
            (Form::Unit, NormalizedValue::Unit) => {}
            (Form::Bool, NormalizedValue::Bool(value)) => bytes[0] = u8::from(*value),
            (Form::I64, NormalizedValue::I64(value)) => bytes.copy_from_slice(&value.to_le_bytes()),
            (
                Form::Pointer(ty),
                NormalizedValue::RegionPointer {
                    ty: actual,
                    pointer,
                },
            ) if ty == actual && pointer.belongs_to(identity) => {
                bytes.copy_from_slice(&pointer.offset().to_le_bytes())
            }
            (
                Form::Record(fields),
                NormalizedValue::Record(NormalizedRecord::Structural { fields: values }),
            ) if fields.len() == values.len() => {
                for ((name, offset, layout), (actual, value)) in fields.iter().zip(values.iter()) {
                    if name != actual {
                        return Err(error());
                    }
                    layout.write(
                        value,
                        identity,
                        &mut bytes[*offset..*offset + layout.size],
                        control,
                    )?;
                }
            }
            _ => return Err(error()),
        }
        Ok(())
    }

    pub(super) fn decode(
        &self,
        bytes: &[u8],
        identity: runtime::Identity,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        if bytes.len() != self.size {
            return Err(error());
        }
        Ok(match &self.form {
            Form::Unit => NormalizedValue::Unit,
            Form::Bool => NormalizedValue::Bool(match bytes[0] {
                0 => false,
                1 => true,
                _ => return Err(error()),
            }),
            Form::I64 => {
                NormalizedValue::I64(i64::from_le_bytes(bytes.try_into().map_err(|_| error())?))
            }
            Form::Pointer(ty) => NormalizedValue::RegionPointer {
                ty: *ty,
                pointer: runtime::Pointer::in_region(
                    identity,
                    u32::from_le_bytes(bytes.try_into().map_err(|_| error())?),
                ),
            },
            Form::Record(fields) => {
                reserve(
                    (fields.len() * std::mem::size_of::<(Name, NormalizedValue)>()
                        + std::mem::size_of::<Vec<(Name, NormalizedValue)>>()
                        + 2 * std::mem::size_of::<usize>()) as u64,
                )?;
                control.check()?;
                let mut values = Vec::new();
                values
                    .try_reserve_exact(fields.len())
                    .map_err(|_| error())?;
                for (name, offset, layout) in fields {
                    reserve(name.as_str().len() as u64)?;
                    let value = layout.decode(
                        &bytes[*offset..*offset + layout.size],
                        identity,
                        control,
                        reserve,
                    )?;
                    values.push((name.clone(), value));
                }
                NormalizedValue::Record(NormalizedRecord::Structural {
                    fields: Arc::new(values),
                })
            }
        })
    }
}

fn align_up(size: usize, alignment: usize) -> Result<usize, ExecutionError> {
    size.checked_add(alignment - 1)
        .map(|size| size & !(alignment - 1))
        .ok_or_else(error)
}

fn fingerprint(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    region: TypeObjectDigest,
    hasher: &mut blake3::Hasher,
    remaining: &mut u64,
    control: &ExecutionControl,
    depth: usize,
) -> Result<(), ExecutionError> {
    control.check()?;
    *remaining = remaining.checked_sub(1).ok_or_else(error)?;
    if ty == region {
        hasher.update(&[0]);
        return Ok(());
    }
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(error());
    }
    let object = types.get(&ty).ok_or_else(error)?;
    match &object.form {
        TypeForm::RelPtr { region, item } | TypeForm::RegionSlice { region, item } => {
            hasher.update(&[if matches!(object.form, TypeForm::RelPtr { .. }) {
                1
            } else {
                2
            }]);
            fingerprint(types, *item, *region, hasher, remaining, control, depth + 1)?;
        }
        TypeForm::StructuralRecord { fields } => {
            hasher.update(&[3]);
            hasher.update(&(fields.len() as u64).to_le_bytes());
            for field in fields {
                hasher.update(&(field.name.as_str().len() as u64).to_le_bytes());
                hasher.update(field.name.as_str().as_bytes());
                fingerprint(
                    types,
                    field.ty,
                    region,
                    hasher,
                    remaining,
                    control,
                    depth + 1,
                )?;
            }
        }
        TypeForm::Unit => {
            hasher.update(&[4]);
        }
        TypeForm::Bool => {
            hasher.update(&[5]);
        }
        TypeForm::I64 => {
            hasher.update(&[6]);
        }
        _ => return Err(error()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::{StructuralTypeField, encode_type_object};

    #[test]
    fn region_pointer_layout_fingerprint_delimits_nested_record_fields() {
        let mut types = BTreeMap::new();
        let mut insert = |form| {
            let object = TypeObject::new(form).unwrap();
            let (id, _) = encode_type_object(&object).unwrap();
            types.insert(id, object);
            id
        };
        let region = insert(TypeForm::Unit);
        let i64_type = insert(TypeForm::I64);
        let field = |name, ty| StructuralTypeField {
            name: Name::new(name).unwrap(),
            ty,
        };
        let inner_one = insert(TypeForm::StructuralRecord {
            fields: vec![field("b", i64_type)],
        });
        let inner_two = insert(TypeForm::StructuralRecord {
            fields: vec![field("b", i64_type), field("c", i64_type)],
        });
        let left = insert(TypeForm::StructuralRecord {
            fields: vec![field("a", inner_one), field("c", i64_type)],
        });
        let right = insert(TypeForm::StructuralRecord {
            fields: vec![field("a", inner_two)],
        });
        let fingerprint_of = |ty| {
            let mut hasher = blake3::Hasher::new();
            let mut remaining = 100;
            fingerprint(
                &types,
                ty,
                region,
                &mut hasher,
                &mut remaining,
                &ExecutionControl::uncancelled(),
                0,
            )
            .unwrap();
            hasher.finalize()
        };
        assert_ne!(fingerprint_of(left), fingerprint_of(right));
    }
}
