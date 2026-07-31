use lkjscript_core::{
    SemanticPayload, SemanticValue, StructuralFieldPath, StructuralKind, StructuralLayoutKind,
    StructuralProjection,
};

use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: lkjscript_core::Op) -> Result<()> {
    match op {
        lkjscript_core::Op::StructuralBorrow | lkjscript_core::Op::StructuralBorrowMut => {
            borrow(vm, op == lkjscript_core::Op::StructuralBorrowMut)
        }
        lkjscript_core::Op::StructuralPublish => publish(vm),
        lkjscript_core::Op::StructuralCopy => copy_owner(vm),
        _ => Err(Error::msg("structural publish opcode dispatch mismatch")),
    }
}

fn borrow<J: RuntimeTier>(vm: &mut Vm<'_, J>, exclusive: bool) -> Result<()> {
    let view_representation = StructuralRepresentationId::new(vm.read_u16()?);
    let view_type =
        representation_type(vm.chunk, view_representation, StructuralValueCategory::View)?;
    let owner_value = vm.pop()?;
    let (owner, record) = invocation(vm)?.owner(owner_value)?;
    if record.value_type != view_type
        || !same_representation_type(vm.chunk, record.representation, view_representation)?
    {
        return Err(Error::msg(
            "structural borrow representation does not match its owner",
        ));
    }
    let view = invocation_mut(vm)?
        .runtime
        .borrow_projected(
            owner,
            record.value_type,
            StructuralProjection::Field {
                path: StructuralFieldPath::root(),
                expected: record.value_type,
            },
            exclusive,
        )
        .map_err(map_value_error)?;
    match invocation_mut(vm)?.register_view(view, view_representation, view_type, false) {
        Ok(value) => {
            vm.push(value);
            Ok(())
        }
        Err(error) => {
            let _ = invocation_mut(vm)?.runtime.end_view(view);
            Err(error)
        }
    }
}

fn copy_owner<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let expected_representation = StructuralRepresentationId::new(vm.read_u16()?);
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
    let copied = invocation_mut(vm)?
        .runtime
        .clone_owned(owner, expected_type)
        .map_err(map_value_error)?;
    let value =
        invocation_mut(vm)?.register_owner(copied, expected_representation, expected_type)?;
    vm.push(value);
    Ok(())
}

fn publish<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let expected_representation = StructuralRepresentationId::new(vm.read_u16()?);
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
        let record = invocation_mut(vm)?
            .owners
            .get_mut(&key.get())
            .ok_or_else(|| Error::msg("structural publish owner disappeared"))?;
        record.representation = expected_representation;
        vm.push(input);
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
