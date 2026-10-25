//! Fixed cell representation. Both evaluators supply independently admitted type
//! inventories. Flat layout nodes and explicit worklists keep the whole codec
//! lifecycle independent of the host call stack.
use super::value::{NormalizedRecord, NormalizedValue, RawValue, ValueOrigin};
use crate::platform::execution::{
    ExecutionControl, ExecutionError,
    region::{RegionLayout, runtime},
};
use crate::platform::kernel::{Name, TypeForm, TypeObject, TypeObjectDigest};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

pub(super) type Arena = runtime::Arena<ValueOrigin>;

pub(super) struct Layout {
    pub(super) storage: RegionLayout,
    nodes: Vec<Node>,
    root: usize,
}

struct Node {
    form: Form,
    size: usize,
    alignment: usize,
    key: [u8; 32],
}

struct Field {
    name: Name,
    offset: usize,
    node: usize,
}

enum Form {
    Unit,
    Bool,
    I64,
    Pointer(TypeObjectDigest),
    Record(Vec<Field>),
}

pub(super) fn error() -> ExecutionError {
    ExecutionError::new(
        crate::platform::execution::ExecutionFailureClass::Trap,
        "normalized_region_layout",
        "region value differs from its exact fixed layout or brand",
    )
}

fn storage_error() -> ExecutionError {
    ExecutionError::resource(
        "normalized_region_storage",
        "region codec cannot represent or reserve its work storage",
    )
}

fn reserve_slots<T>(
    values: &mut Vec<T>,
    additional: usize,
    control: &ExecutionControl,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    control.check()?;
    let required = values.len().checked_add(additional).ok_or_else(storage_error)?;
    if required <= values.capacity() {
        return Ok(());
    }
    std::alloc::Layout::array::<T>(required).map_err(|_| storage_error())?;
    let capacity = required.checked_next_power_of_two()
        .filter(|capacity| std::alloc::Layout::array::<T>(*capacity).is_ok())
        .unwrap_or(required);
    let bytes = capacity.checked_mul(std::mem::size_of::<T>())
        .and_then(|bytes| u64::try_from(bytes).ok()).ok_or_else(storage_error)?;
    reserve(bytes)?;
    control.check()?;
    values.try_reserve_exact(capacity - values.len()).map_err(|_| storage_error())
}

fn push<T>(
    values: &mut Vec<T>,
    value: T,
    control: &ExecutionControl,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    reserve_slots(values, 1, control, reserve)?;
    values.push(value);
    Ok(())
}

pub(super) fn check_plain(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    control.check()?;
    let mut pending = Vec::new();
    pending.try_reserve(1).map_err(|_| storage_error())?;
    pending.push((ty, value));
    while let Some((ty, value)) = pending.pop() {
        control.check()?;
        match (&types.get(&ty).ok_or_else(error)?.form, value) {
            (TypeForm::Unit, NormalizedValue::Unit)
            | (TypeForm::Bool, NormalizedValue::Bool(_))
            | (TypeForm::I64, NormalizedValue::I64(_)) => {}
            (TypeForm::RelPtr { .. }, NormalizedValue::RegionPointer { ty: actual, .. })
                if *actual == ty => {}
            (
                TypeForm::StructuralRecord { fields },
                NormalizedValue::Record(NormalizedRecord::Structural { fields: values }),
            ) if fields.len() == values.len() => {
                pending.try_reserve(fields.len()).map_err(|_| storage_error())?;
                for (field, (name, value)) in fields.iter().zip(values.iter()).rev() {
                    if field.name != *name {
                        return Err(error());
                    }
                    pending.push((field.ty, value));
                }
            }
            _ => return Err(error()),
        }
    }
    Ok(())
}

impl Layout {
    pub(super) fn derive(
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        ty: TypeObjectDigest,
        region: TypeObjectDigest,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        enum Task { Enter(TypeObjectDigest), Record(TypeObjectDigest, usize) }
        let mut pending = Vec::new();
        let mut completed = Vec::new();
        let mut active = HashSet::new();
        let mut known = HashMap::new();
        let mut nodes = Vec::new();
        push(&mut pending, Task::Enter(ty), control, reserve)?;
        while let Some(task) = pending.pop() {
            control.check()?;
            let (ty, node) = match task {
                Task::Enter(ty) => {
                    if let Some(&node) = known.get(&ty) {
                        push(&mut completed, node, control, reserve)?;
                        continue;
                    }
                    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.region.cell.v2");
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
                        TypeForm::RelPtr { region: actual, item } if *actual == region => {
                            hasher.update(&[3]);
                            fingerprint(types, *item, region, &mut hasher, control, reserve)?;
                            (Form::Pointer(ty), 8, 8)
                        }
                        TypeForm::StructuralRecord { fields } => {
                            active.try_reserve(1).map_err(|_| storage_error())?;
                            if !active.insert(ty) {
                                return Err(error());
                            }
                            let first = completed.len();
                            push(&mut pending, Task::Record(ty, first), control, reserve)?;
                            reserve_slots(&mut pending, fields.len(), control, reserve)?;
                            for field in fields.iter().rev() {
                                pending.push(Task::Enter(field.ty));
                            }
                            continue;
                        }
                        _ => return Err(error()),
                    };
                    (ty, Node { form, size, alignment, key: *hasher.finalize().as_bytes() })
                }
                Task::Record(ty, first) => {
                    let TypeForm::StructuralRecord { fields } =
                        &types.get(&ty).ok_or_else(error)?.form else { return Err(error()); };
                    if completed.len().checked_sub(first) != Some(fields.len()) {
                        return Err(error());
                    }
                    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.region.cell.v2");
                    hasher.update(&[4]);
                    hasher.update(&(fields.len() as u64).to_le_bytes());
                    let mut entries = Vec::new();
                    reserve_slots(&mut entries, fields.len(), control, reserve)?;
                    let mut size = 0usize;
                    let mut alignment = 1usize;
                    for (field, &node) in fields.iter().zip(&completed[first..]) {
                        let child: &Node = nodes.get(node).ok_or_else(error)?;
                        alignment = alignment.max(child.alignment);
                        size = align_up(size, child.alignment)?;
                        hasher.update(&(field.name.as_str().len() as u64).to_le_bytes());
                        hasher.update(field.name.as_str().as_bytes());
                        hasher.update(&child.key);
                        reserve(field.name.as_str().len() as u64)?;
                        control.check()?;
                        entries.push(Field { name: field.name.clone(), offset: size, node });
                        size = size.checked_add(child.size).ok_or_else(storage_error)?;
                    }
                    completed.truncate(first);
                    active.remove(&ty);
                    (ty, Node {
                        form: Form::Record(entries),
                        size: align_up(size, alignment)?,
                        alignment,
                        key: *hasher.finalize().as_bytes(),
                    })
                }
            };
            let index = nodes.len();
            push(&mut nodes, node, control, reserve)?;
            known.try_reserve(1).map_err(|_| storage_error())?;
            known.insert(ty, index);
            push(&mut completed, index, control, reserve)?;
        }
        if completed.len() != 1 {
            return Err(error());
        }
        let root = completed[0];
        let node = &nodes[root];
        let storage = RegionLayout::new(node.key,
            u64::try_from(node.size).map_err(|_| storage_error())?,
            u64::try_from(node.alignment).map_err(|_| storage_error())?)?;
        Ok(Self { storage, nodes, root })
    }

    pub(super) fn encode(
        &self,
        value: &NormalizedValue,
        identity: runtime::Identity,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Vec<u8>, ExecutionError> {
        control.check()?;
        let size = self.nodes[self.root].size;
        reserve(size as u64)?;
        control.check()?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(size).map_err(|_| storage_error())?;
        bytes.resize(size, 0);
        let mut pending = Vec::new();
        push(&mut pending, (self.root, value, 0usize), control, reserve)?;
        while let Some((node, value, offset)) = pending.pop() {
            control.check()?;
            let node = self.nodes.get(node).ok_or_else(error)?;
            let end = offset.checked_add(node.size).ok_or_else(error)?;
            let output = bytes.get_mut(offset..end).ok_or_else(error)?;
            match (&node.form, value) {
                (Form::Unit, NormalizedValue::Unit) => {}
                (Form::Bool, NormalizedValue::Bool(value)) => output[0] = u8::from(*value),
                (Form::I64, NormalizedValue::I64(value)) => output.copy_from_slice(&value.to_le_bytes()),
                (Form::Pointer(ty), NormalizedValue::RegionPointer { ty: actual, pointer })
                    if ty == actual && pointer.belongs_to(identity) => {
                        output.copy_from_slice(&pointer.offset().to_le_bytes());
                    }
                (Form::Record(fields), NormalizedValue::Record(NormalizedRecord::Structural { fields: values }))
                    if fields.len() == values.len() => {
                        reserve_slots(&mut pending, fields.len(), control, reserve)?;
                        for (field, (name, value)) in fields.iter().zip(values.iter()).rev() {
                            if field.name != *name { return Err(error()); }
                            let child_offset = offset.checked_add(field.offset).ok_or_else(error)?;
                            pending.push((field.node, value, child_offset));
                        }
                    }
                _ => return Err(error()),
            }
        }
        Ok(bytes)
    }

    pub(super) fn decode(
        &self,
        bytes: &[u8],
        identity: runtime::Identity,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<NormalizedValue, ExecutionError> {
        enum Task { Read(usize, usize), Record(usize, usize) }
        control.check()?;
        if bytes.len() != self.nodes[self.root].size { return Err(error()); }
        let mut pending = Vec::new();
        // Guards release even partially decoded deep values iteratively on error.
        let mut values: Vec<RawValue> = Vec::new();
        push(&mut pending, Task::Read(self.root, 0), control, reserve)?;
        while let Some(task) = pending.pop() {
            control.check()?;
            let value = match task {
                Task::Read(index, offset) => {
                    let node = self.nodes.get(index).ok_or_else(error)?;
                    let end = offset.checked_add(node.size).ok_or_else(error)?;
                    let input = bytes.get(offset..end).ok_or_else(error)?;
                    match &node.form {
                        Form::Unit => NormalizedValue::Unit,
                        Form::Bool => NormalizedValue::Bool(match input.first() {
                            Some(0) => false,
                            Some(1) => true,
                            _ => return Err(error()),
                        }),
                        Form::I64 => NormalizedValue::I64(i64::from_le_bytes(input.try_into().map_err(|_| error())?)),
                        Form::Pointer(ty) => NormalizedValue::RegionPointer {
                            ty: *ty,
                            pointer: runtime::Pointer::in_region(identity,
                                u64::from_le_bytes(input.try_into().map_err(|_| error())?)),
                        },
                        Form::Record(fields) => {
                            let first = values.len();
                            push(&mut pending, Task::Record(index, first), control, reserve)?;
                            reserve_slots(&mut pending, fields.len(), control, reserve)?;
                            for field in fields.iter().rev() {
                                pending.push(Task::Read(field.node,
                                    offset.checked_add(field.offset).ok_or_else(error)?));
                            }
                            continue;
                        }
                    }
                }
                Task::Record(index, first) => {
                    let Form::Record(fields) = &self.nodes.get(index).ok_or_else(error)?.form
                        else { return Err(error()); };
                    if values.len().checked_sub(first) != Some(fields.len()) { return Err(error()); }
                    let metadata = std::mem::size_of::<Vec<(Name, NormalizedValue)>>()
                        .checked_add(2 * std::mem::size_of::<usize>()).ok_or_else(storage_error)?;
                    reserve(metadata as u64)?;
                    let mut record = Vec::new();
                    reserve_slots(&mut record, fields.len(), control, reserve)?;
                    for field in fields {
                        reserve(field.name.as_str().len() as u64)?;
                    }
                    control.check()?;
                    // All fallible work finishes before moving children out of guards.
                    for (field, value) in fields.iter().zip(values.drain(first..)) {
                        record.push((field.name.clone(), value.into_raw()));
                    }
                    NormalizedValue::Record(NormalizedRecord::Structural { fields: Arc::new(record) })
                }
            };
            push(&mut values, RawValue::new(value), control, reserve)?;
        }
        if values.len() != 1 { return Err(error()); }
        values.pop().map(RawValue::into_raw).ok_or_else(error)
    }
}

fn align_up(size: usize, alignment: usize) -> Result<usize, ExecutionError> {
    size.checked_add(alignment - 1)
        .map(|size| size & !(alignment - 1)).ok_or_else(storage_error)
}

fn fingerprint(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    region: TypeObjectDigest,
    hasher: &mut blake3::Hasher,
    control: &ExecutionControl,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    control.check()?;
    enum Task {
        Type(TypeObjectDigest, TypeObjectDigest),
        Finish(TypeObjectDigest, TypeObjectDigest, usize),
    }
    let mut pending = Vec::new();
    let mut active = HashSet::new();
    let mut completed: Vec<[u8; 32]> = Vec::new();
    let mut known: HashMap<(TypeObjectDigest, TypeObjectDigest), [u8; 32]> = HashMap::new();
    push(&mut pending, Task::Type(ty, region), control, reserve)?;
    while let Some(task) = pending.pop() {
        control.check()?;
        let (ty, region, key) = match task {
            Task::Type(ty, region) => {
                if let Some(&key) = known.get(&(ty, region)) {
                    push(&mut completed, key, control, reserve)?;
                    continue;
                }
                let mut node = blake3::Hasher::new_derive_key("lkjscript.region.type.v2");
                if ty == region {
                    node.update(&[0]);
                } else {
                    let object = types.get(&ty).ok_or_else(error)?;
                    match &object.form {
                        TypeForm::RelPtr { region: next_region, item }
                        | TypeForm::RegionSlice { region: next_region, item } => {
                            active.try_reserve(1).map_err(|_| storage_error())?;
                            if !active.insert((ty, region)) { return Err(error()); }
                            push(&mut pending, Task::Finish(ty, region, completed.len()), control, reserve)?;
                            push(&mut pending, Task::Type(*item, *next_region), control, reserve)?;
                            continue;
                        }
                        TypeForm::StructuralRecord { fields } => {
                            active.try_reserve(1).map_err(|_| storage_error())?;
                            if !active.insert((ty, region)) { return Err(error()); }
                            push(&mut pending, Task::Finish(ty, region, completed.len()), control, reserve)?;
                            reserve_slots(&mut pending, fields.len(), control, reserve)?;
                            for field in fields.iter().rev() {
                                pending.push(Task::Type(field.ty, region));
                            }
                            continue;
                        }
                        TypeForm::Unit => { node.update(&[4]); }
                        TypeForm::Bool => { node.update(&[5]); }
                        TypeForm::I64 => { node.update(&[6]); }
                        _ => return Err(error()),
                    }
                }
                (ty, region, *node.finalize().as_bytes())
            }
            Task::Finish(ty, region, first) => {
                let object = types.get(&ty).ok_or_else(error)?;
                let mut node = blake3::Hasher::new_derive_key("lkjscript.region.type.v2");
                match &object.form {
                    TypeForm::RelPtr { .. } | TypeForm::RegionSlice { .. } => {
                        if completed.len().checked_sub(first) != Some(1) { return Err(error()); }
                        node.update(&[if matches!(object.form, TypeForm::RelPtr { .. }) { 1 } else { 2 }]);
                        node.update(&completed[first]);
                    }
                    TypeForm::StructuralRecord { fields } => {
                        if completed.len().checked_sub(first) != Some(fields.len()) { return Err(error()); }
                        node.update(&[3]);
                        node.update(&(fields.len() as u64).to_le_bytes());
                        for (field, key) in fields.iter().zip(&completed[first..]) {
                            node.update(&(field.name.as_str().len() as u64).to_le_bytes());
                            node.update(field.name.as_str().as_bytes());
                            node.update(key);
                        }
                    }
                    _ => return Err(error()),
                }
                completed.truncate(first);
                active.remove(&(ty, region));
                (ty, region, *node.finalize().as_bytes())
            }
        };
        known.try_reserve(1).map_err(|_| storage_error())?;
        known.insert((ty, region), key);
        push(&mut completed, key, control, reserve)?;
    }
    if completed.len() != 1 { return Err(error()); }
    hasher.update(&completed[0]);
    Ok(())
}

#[cfg(test)]
mod tests;
