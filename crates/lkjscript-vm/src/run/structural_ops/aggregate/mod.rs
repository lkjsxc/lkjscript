use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StaticStructuralLeaf,
    StructuralFieldPath, StructuralFieldRoute, StructuralKind, StructuralNodeView,
    StructuralPayloadRef, StructuralProjection, VariantId,
};

use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: lkjscript_core::Op) -> Result<()> {
    match op {
        lkjscript_core::Op::StructuralAggregateFieldBorrow => field_borrow(vm),
        lkjscript_core::Op::StructuralAggregateFieldCopy => field_copy(vm),
        lkjscript_core::Op::StructuralAggregateTag => tag(vm),
        lkjscript_core::Op::StructuralAggregateConsumePayload => consume_payload(vm),
        lkjscript_core::Op::StructuralStringUtf8View => string_utf8_view(vm),
        _ => Err(Error::msg("structural aggregate opcode dispatch mismatch")),
    }
}

fn tag<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let expected_representation = StructuralRepresentationId::new(vm.read_u16()?);
    let source = vm.pop()?;
    let (owner, record) = invocation(vm)?.owner(source)?;
    require_owner_representation(vm.chunk, record, expected_representation)?;
    let node = invocation(vm)?
        .runtime
        .value_node(owner, record.value_type)
        .map_err(map_value_error)?;
    let StructuralNodeView::Enum { tag, .. } = node.payload() else {
        return Err(Error::msg("structural aggregate tag expects enum payload"));
    };
    vm.push(Value::from_i64(i64::from(tag)));
    Ok(())
}

fn consume_payload<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let index = usize::from(vm.read_u16()?);
    let reference = *vm
        .chunk
        .structural_payloads()
        .get(index)
        .ok_or_else(|| Error::msg("structural payload reference is stale"))?;
    let source = vm.pop()?;
    let (owner, record) = invocation(vm)?.owner(source)?;
    require_owner_representation(vm.chunk, record, reference.representation)?;
    preflight_payload(vm, owner, record.value_type, reference)?;
    let semantic = invocation_mut(vm)?
        .runtime
        .export_semantic(owner, record.value_type)
        .map_err(map_value_error)?;
    invocation_mut(vm)?.owners.remove(&owner.get());
    places::clear_consumed_owner(vm, owner.get());
    let SemanticPayload::Enum {
        mut active_payload, ..
    } = semantic.payload
    else {
        return Err(Error::msg("structural payload export changed shape"));
    };
    let payload = active_payload
        .pop()
        .ok_or_else(|| Error::msg("structural payload export is empty"))?;
    let result = match reference.result.route {
        StructuralFieldRoute::Copy => semantic_to_value(vm.chunk, &payload)?,
        StructuralFieldRoute::Structural(type_id) => {
            let representation = owner_representation_for_type(vm.chunk, type_id)?;
            let expected = reference
                .result
                .runtime_type
                .ok_or_else(|| Error::msg("structural payload lacks exact runtime type"))?;
            let key = invocation_mut(vm)?
                .runtime
                .publish_owned(payload)
                .map_err(|failure| map_value_error(failure.error))?;
            invocation_mut(vm)?.register_owner(key, representation, expected)?
        }
        StructuralFieldRoute::Unique
        | StructuralFieldRoute::Resource
        | StructuralFieldRoute::LegacyHeap => {
            return Err(Error::msg(
                "structural payload consume crosses an unsupported ownership route",
            ));
        }
    };
    vm.push(result);
    Ok(())
}

fn string_utf8_view<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let view_representation = StructuralRepresentationId::new(vm.read_u16()?);
    let expected =
        representation_type(vm.chunk, view_representation, StructuralValueCategory::View)?;
    if expected.kind != StructuralKind::String {
        return Err(Error::msg("structural UTF-8 view metadata is not string"));
    }
    let source = vm.pop()?;
    let (owner, record) = invocation(vm)?.owner(source)?;
    require_owner_representation(vm.chunk, record, view_representation)?;
    let node = invocation(vm)?
        .runtime
        .value_node(owner, record.value_type)
        .map_err(map_value_error)?;
    let StructuralNodeView::Bytes(bytes) = node.payload() else {
        return Err(Error::msg("structural UTF-8 view expects string payload"));
    };
    let end = u32::try_from(bytes.len()).map_err(|_| {
        Error::resource(
            ResourceLimitKind::HeapBytes,
            "structural UTF-8 view range exceeds u32",
        )
    })?;
    let view = invocation_mut(vm)?
        .runtime
        .borrow_projected(
            owner,
            record.value_type,
            StructuralProjection::Utf8 {
                path: StructuralFieldPath::root(),
                expected,
                start: 0,
                end,
            },
            false,
        )
        .map_err(map_value_error)?;
    let value = register_view_or_end(vm, view, view_representation, expected, true)?;
    vm.push(value);
    Ok(())
}

include!("fields.rs");
include!("support.rs");
