use lkjscript_core::{
    SemanticPayload, SemanticValue, StructuralFieldPath, StructuralKind, StructuralLayoutKind,
    StructuralProjection, StructuralStorage,
};

use super::*;

mod borrow;
mod witness;
use borrow::borrow;
use witness::{compare, dispose_owner, independent_owner};

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: lkjscript_core::Op) -> Result<()> {
    match op {
        lkjscript_core::Op::StructuralBorrow | lkjscript_core::Op::StructuralBorrowMut => {
            borrow(vm, op == lkjscript_core::Op::StructuralBorrowMut)
        }
        lkjscript_core::Op::StructuralPublish => publish(vm),
        lkjscript_core::Op::StructuralCopy => copy_owner(vm),
        lkjscript_core::Op::MemoryWitnessIndependentOwner => independent_owner(vm),
        lkjscript_core::Op::MemoryWitnessCompare => compare(vm),
        lkjscript_core::Op::MemoryWitnessDispose => dispose_owner(vm),
        _ => Err(Error::msg("structural publish opcode dispatch mismatch")),
    }
}

fn copy_owner<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let expected_representation = StructuralRepresentationId::new(vm.read_u64()?);
    let expected_type = representation_type(
        vm.chunk,
        expected_representation,
        StructuralValueCategory::Owner,
    )?;
    let mode = vm
        .chunk
        .structural_representations()
        .get(expected_representation.index())
        .and_then(|representation| {
            vm.chunk
                .structural_types()
                .get(representation.type_id.index())
        })
        .map(|metadata| metadata.mode)
        .ok_or_else(|| Error::msg("structural copy type metadata is missing"))?;
    if mode == lkjscript_core::StructuralTypeMode::Affine {
        return Err(Error::msg(
            "structural copy cannot duplicate an affine owner",
        ));
    }
    let source = vm.pop()?;
    let (owner, record) = invocation(vm)?.owner(source)?;
    if record.value_type != expected_type
        || !same_representation_type(vm.chunk, record.representation, expected_representation)?
    {
        return Err(Error::msg("structural copy owner type mismatch"));
    }
    let storage = representation(vm.chunk, expected_representation)?.storage;
    let copied = if storage == StructuralStorage::SealedRegion {
        invocation_mut(vm)?
            .runtime
            .acquire_sealed(owner, expected_type)
    } else {
        invocation_mut(vm)?
            .runtime
            .clone_owned(owner, expected_type)
    }
    .map_err(map_value_error)?;
    let value =
        invocation_mut(vm)?.register_owner(copied, expected_representation, expected_type)?;
    vm.push(value);
    Ok(())
}

fn publish<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let expected_representation = StructuralRepresentationId::new(vm.read_u64()?);
    let expected_type = representation_type(
        vm.chunk,
        expected_representation,
        StructuralValueCategory::Owner,
    )?;
    let input = vm.pop()?;
    let host_owner =
        invocation(vm).is_ok_and(|structural| values::is_host_owner(structural, input));
    if let Some(key) = input.as_structural_root().filter(|key| {
        invocation(vm).is_ok_and(|structural| structural.owners.contains_key(&key.get()))
    }) {
        let (_, record) = invocation(vm)?.owner(input)?;
        if record.value_type != expected_type
            || !same_representation_type(vm.chunk, record.representation, expected_representation)?
        {
            return Err(Error::msg("structural publish owner type mismatch"));
        }
        let source_storage = representation(vm.chunk, record.representation)?.storage;
        let expected_storage = representation(vm.chunk, expected_representation)?.storage;
        let published = match (source_storage, expected_storage) {
            (StructuralStorage::UniqueStructural, StructuralStorage::SealedRegion) => {
                invocation_mut(vm)?
                    .runtime
                    .seal_owned(key, expected_type)
                    .map_err(map_value_error)?
                    .owner
            }
            (left, right) if left == right => key,
            _ => return Err(Error::msg("structural publish storage route mismatch")),
        };
        invocation_mut(vm)?.owners.remove(&key.get());
        let value = invocation_mut(vm)?.register_owner(
            published,
            expected_representation,
            expected_type,
        )?;
        vm.push(value);
        return Ok(());
    }
    if input.as_structural_root().is_some() && !host_owner {
        return Err(Error::msg(
            "structural publish received a stale or forged owner",
        ));
    }
    let semantic = semantic_from_input(vm, input, expected_representation, expected_type)?;
    let key = invocation_mut(vm)?
        .runtime
        .publish_owned(semantic)
        .map_err(|failure| map_value_error(failure.error))?;
    let key = if representation(vm.chunk, expected_representation)?.storage
        == StructuralStorage::SealedRegion
    {
        match invocation_mut(vm)?.runtime.seal_owned(key, expected_type) {
            Ok(sealed) => sealed.owner,
            Err(error) => {
                let _ = invocation_mut(vm)?
                    .runtime
                    .dispose_owner(key, expected_type);
                return Err(map_value_error(error));
            }
        }
    } else {
        key
    };
    let owner = invocation_mut(vm)?.register_owner(key, expected_representation, expected_type)?;
    if host_owner {
        values::drop_host_owner(vm, input)?;
    }
    vm.push(owner);
    Ok(())
}

fn semantic_from_input<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    value: Value,
    representation_id: StructuralRepresentationId,
    expected: StructuralType,
) -> Result<SemanticValue> {
    let representation = representation(vm.chunk, representation_id)?;
    let layout = vm
        .chunk
        .structural_layouts()
        .get(representation.layout.index())
        .filter(|item| item.id == representation.layout)
        .ok_or_else(|| Error::msg("structural publish layout metadata is stale"))?;
    let payload = match &layout.kind {
        StructuralLayoutKind::String if expected.kind == StructuralKind::String => {
            SemanticPayload::String(values::copy_string(vm, value)?.into_bytes())
        }
        StructuralLayoutKind::Path if expected.kind == StructuralKind::Path => {
            SemanticPayload::Path(values::copy_path(vm, value)?)
        }
        StructuralLayoutKind::Product { .. } | StructuralLayoutKind::Enum { .. } => {
            return Err(Error::msg(
                "unsupported aggregates cannot publish structural values",
            ));
        }
        _ => {
            return Err(Error::msg(
                "structural publish input does not match exact representation metadata",
            ));
        }
    };
    Ok(SemanticValue::new(expected, payload))
}
