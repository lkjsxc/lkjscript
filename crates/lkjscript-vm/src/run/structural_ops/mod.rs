use std::collections::BTreeMap;

use lkjscript_core::{
    Error, ResourceLimitKind, Result, StructuralDestinationId, StructuralDestinationKey,
    StructuralRepresentationId, StructuralRootTableError, StructuralType, StructuralValueCategory,
    StructuralValueError, StructuralValueKey, StructuralValueRuntime, StructuralValueRuntimeLimits,
    StructuralViewKey, ValidatedChunk, Value,
};

use super::{unique, RuntimeTier, Vm};

mod adapter;
mod aggregate;
mod bytes;
mod cleanup;
mod destination;
mod locals;
mod places;
mod publish;
mod values;

pub(in crate::run) use adapter::{
    adapter_is_variant, adapter_take_field, drop_registered_owner, drop_resource_adapter,
    publish_numeric_result, publish_option, publish_system_result, publish_system_utf8_result,
    publish_utf8_result, value_from_runtime, HostValue, HostValueType,
};
pub(super) use bytes::{byte_at, end_byte_view, is_byte_view, len, read_u32_little_endian};
pub(super) use cleanup::{
    cleanup_failure_action, cleanup_failure_roots, export_return, prepare_exit, teardown,
};
pub(super) use locals::{
    call_memory_witnesses, commit_call_arguments, initialize_call_places, prepare_return,
    restore_handoffs,
};
pub(super) use values::{
    copy_path, copy_string, export_plain_return, publish_string, semantic_snapshot,
};

include!("invocation.rs");

pub(super) fn handles(op: u8) -> bool {
    matches!(
        lkjscript_core::Op::from_byte(op),
        Some(
            lkjscript_core::Op::StoreStructuralLocal
                | lkjscript_core::Op::TakeStructuralLocal
                | lkjscript_core::Op::LoadStructuralViewLocal
                | lkjscript_core::Op::EndStructuralBorrowLocal
                | lkjscript_core::Op::LoadStructuralOwnerLocal
                | lkjscript_core::Op::StructuralPlaceInit
                | lkjscript_core::Op::StructuralMove
                | lkjscript_core::Op::StructuralDropPlace
                | lkjscript_core::Op::StructuralPlaceEnd
                | lkjscript_core::Op::StructuralBorrow
                | lkjscript_core::Op::StructuralBorrowMut
                | lkjscript_core::Op::StructuralPublish
                | lkjscript_core::Op::StructuralDestinationCreate
                | lkjscript_core::Op::StructuralDestinationFieldInit
                | lkjscript_core::Op::StructuralDestinationFinish
                | lkjscript_core::Op::StructuralDestinationAbort
                | lkjscript_core::Op::StructuralAggregateFieldBorrow
                | lkjscript_core::Op::StructuralAggregateFieldCopy
                | lkjscript_core::Op::StructuralAggregateTag
                | lkjscript_core::Op::StructuralAggregateConsumePayload
                | lkjscript_core::Op::StructuralStringUtf8View
                | lkjscript_core::Op::StructuralCopy
        )
    )
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    let op =
        lkjscript_core::Op::from_byte(op).ok_or_else(|| Error::msg("unknown structural opcode"))?;
    match op {
        lkjscript_core::Op::StoreStructuralLocal
        | lkjscript_core::Op::TakeStructuralLocal
        | lkjscript_core::Op::LoadStructuralViewLocal
        | lkjscript_core::Op::EndStructuralBorrowLocal
        | lkjscript_core::Op::LoadStructuralOwnerLocal => locals::dispatch(vm, op),
        lkjscript_core::Op::StructuralPlaceInit
        | lkjscript_core::Op::StructuralMove
        | lkjscript_core::Op::StructuralDropPlace
        | lkjscript_core::Op::StructuralPlaceEnd => places::dispatch(vm, op),
        lkjscript_core::Op::StructuralBorrow
        | lkjscript_core::Op::StructuralBorrowMut
        | lkjscript_core::Op::StructuralPublish
        | lkjscript_core::Op::StructuralCopy => publish::dispatch(vm, op),
        lkjscript_core::Op::StructuralDestinationCreate
        | lkjscript_core::Op::StructuralDestinationFieldInit
        | lkjscript_core::Op::StructuralDestinationFinish
        | lkjscript_core::Op::StructuralDestinationAbort => destination::dispatch(vm, op),
        lkjscript_core::Op::StructuralAggregateFieldBorrow
        | lkjscript_core::Op::StructuralAggregateFieldCopy
        | lkjscript_core::Op::StructuralAggregateTag
        | lkjscript_core::Op::StructuralAggregateConsumePayload
        | lkjscript_core::Op::StructuralStringUtf8View => aggregate::dispatch(vm, op),
        _ => Err(Error::msg("structural opcode dispatch mismatch")),
    }
}

fn invocation<'vm, J: RuntimeTier>(vm: &'vm Vm<'_, J>) -> Result<&'vm StructuralInvocation> {
    vm.structural
        .as_ref()
        .ok_or_else(|| Error::msg("structural opcode lacks invocation runtime"))
}

fn invocation_mut<'vm, J: RuntimeTier>(
    vm: &'vm mut Vm<'_, J>,
) -> Result<&'vm mut StructuralInvocation> {
    vm.structural
        .as_mut()
        .ok_or_else(|| Error::msg("structural opcode lacks invocation runtime"))
}

fn representation(
    chunk: &ValidatedChunk,
    id: StructuralRepresentationId,
) -> Result<&lkjscript_core::StructuralRepresentationMetadata> {
    chunk
        .structural_representations()
        .get(id.index())
        .filter(|item| item.id == id)
        .ok_or_else(|| Error::msg("structural representation metadata is stale"))
}

fn representation_type(
    chunk: &ValidatedChunk,
    id: StructuralRepresentationId,
    category: StructuralValueCategory,
) -> Result<StructuralType> {
    let representation = representation(chunk, id)?;
    if representation.category != category {
        return Err(Error::msg(
            "structural representation has the wrong value category",
        ));
    }
    chunk
        .structural_types()
        .get(representation.type_id.index())
        .filter(|item| item.id == representation.type_id && item.layout == representation.layout)
        .map(|item| item.runtime_type)
        .ok_or_else(|| Error::msg("structural representation has invalid exact type metadata"))
}

fn same_representation_type(
    chunk: &ValidatedChunk,
    left: StructuralRepresentationId,
    right: StructuralRepresentationId,
) -> Result<bool> {
    let left = representation(chunk, left)?;
    let right = representation(chunk, right)?;
    Ok(left.type_id == right.type_id && left.layout == right.layout)
}

fn map_value_error(error: StructuralValueError) -> Error {
    match error {
        StructuralValueError::AllocationFailed
        | StructuralValueError::LimitExceeded(
            lkjscript_core::StructuralValueLimit::PayloadBytes
            | lkjscript_core::StructuralValueLimit::TreeDepth,
        )
        | StructuralValueError::RootTable(StructuralRootTableError::AllocationFailed) => {
            Error::resource(ResourceLimitKind::HeapBytes, error.to_string())
        }
        StructuralValueError::LimitExceeded(_)
        | StructuralValueError::RootTable(StructuralRootTableError::LimitExceeded(_)) => {
            Error::resource(ResourceLimitKind::Allocations, error.to_string())
        }
        _ => Error::msg(error.to_string()),
    }
}
