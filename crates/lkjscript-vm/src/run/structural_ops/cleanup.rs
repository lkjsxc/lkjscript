use lkjscript_core::{
    FailureCleanupAction, OwnedValue, StructuralSnapshotLimits, StructuralValueCategory,
};

use super::*;

pub(in crate::run) fn cleanup_failure_action<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    frame: usize,
    action: FailureCleanupAction,
) -> Option<Result<()>> {
    match action {
        FailureCleanupAction::EndStructuralBorrow {
            local,
            representation,
            ..
        } => Some(cleanup_view(vm, frame, local, representation)),
        FailureCleanupAction::DropStructural {
            local,
            place,
            representation,
        } => Some(cleanup_owner(vm, frame, local, place, representation)),
        FailureCleanupAction::AbortStructuralDestination { local, destination } => {
            Some(cleanup_destination(vm, frame, local, destination))
        }
        FailureCleanupAction::EndBorrow { .. }
        | FailureCleanupAction::DropUnique { .. }
        | FailureCleanupAction::DropResource { .. } => None,
    }
}

fn cleanup_view<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    frame: usize,
    local: u8,
    representation: StructuralRepresentationId,
) -> Result<()> {
    let index = cleanup_local_index(vm, frame, local)?;
    let value = cleanup_local_value(vm, index)?;
    let (key, record) = invocation(vm)?.view(value)?;
    if record.representation != representation {
        return Err(Error::msg(
            "structural failure cleanup view representation mismatch",
        ));
    }
    invocation_mut(vm)?
        .runtime
        .end_view(key)
        .map_err(map_value_error)?;
    invocation_mut(vm)?.views.remove(&key.get());
    vm.stack[index] = Value::INVALID;
    Ok(())
}

fn cleanup_owner<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    frame: usize,
    local: u8,
    place: Option<u8>,
    representation: StructuralRepresentationId,
) -> Result<()> {
    let index = cleanup_local_index(vm, frame, local)?;
    let value = cleanup_local_value(vm, index)?;
    let (key, record) = invocation(vm)?.owner(value)?;
    if record.representation != representation {
        return Err(Error::msg(
            "structural failure cleanup owner representation mismatch",
        ));
    }
    invocation_mut(vm)?
        .runtime
        .dispose_owner(key, record.value_type)
        .map(|_| ())
        .map_err(map_value_error)?;
    invocation_mut(vm)?.owners.remove(&key.get());
    vm.stack[index] = Value::INVALID;
    if let Some(place) = place {
        if let Some(target) = vm
            .frames
            .get_mut(frame)
            .and_then(|frame| frame.unique_places.get_mut(usize::from(place)))
        {
            *target = unique::RuntimePlace::Active {
                owner: None,
                transferred: None,
            };
        }
    }
    Ok(())
}

fn cleanup_destination<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    frame: usize,
    local: u8,
    destination: StructuralDestinationId,
) -> Result<()> {
    let index = cleanup_local_index(vm, frame, local)?;
    let value = cleanup_local_value(vm, index)?;
    let (key, record) = invocation(vm)?.destination(value)?;
    if record.destination != destination {
        return Err(Error::msg(
            "structural failure cleanup destination metadata mismatch",
        ));
    }
    invocation_mut(vm)?
        .runtime
        .abort_destination(key)
        .map_err(map_value_error)?;
    invocation_mut(vm)?.destinations.remove(&key.get());
    vm.stack[index] = Value::INVALID;
    Ok(())
}

fn cleanup_local_index<J: RuntimeTier>(vm: &Vm<'_, J>, frame: usize, local: u8) -> Result<usize> {
    vm.frames
        .get(frame)
        .and_then(|frame| frame.locals_base.checked_add(usize::from(local)))
        .ok_or_else(|| Error::msg("structural failure cleanup lost its frame local"))
}

fn cleanup_local_value<J: RuntimeTier>(vm: &Vm<'_, J>, index: usize) -> Result<Value> {
    let value = vm
        .stack
        .get(index)
        .copied()
        .ok_or_else(|| Error::msg("structural failure cleanup local is out of range"))?;
    if value.is_invalid() {
        return Err(Error::msg(
            "structural failure cleanup local was not restored",
        ));
    }
    Ok(value)
}

pub(in crate::run) fn export_return<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    value: Value,
    representation: StructuralRepresentationId,
) -> Result<OwnedValue> {
    let expected = representation_type(vm.chunk, representation, StructuralValueCategory::Owner)?;
    let (key, record) = invocation(vm)?.owner(value)?;
    if record.value_type != expected
        || !same_representation_type(vm.chunk, record.representation, representation)?
    {
        return Err(Error::msg(
            "returned structural owner representation mismatch",
        ));
    }
    let semantic = invocation_mut(vm)?
        .runtime
        .export_semantic(key, expected)
        .map_err(map_value_error)?;
    invocation_mut(vm)?.owners.remove(&key.get());
    OwnedValue::from_structural(semantic, StructuralSnapshotLimits::DEFAULT)
}

pub(in crate::run) fn cleanup_failure_roots<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let owners = invocation(vm)?
        .owners
        .iter()
        .map(|(word, record)| (*word, record.value_type))
        .collect::<Vec<_>>();
    for (word, value_type) in owners {
        let key = StructuralValueKey::from_word(word)
            .ok_or_else(|| Error::msg("failure structural owner key is malformed"))?;
        invocation_mut(vm)?
            .runtime
            .dispose_owner(key, value_type)
            .map(|_| ())
            .map_err(map_value_error)?;
        invocation_mut(vm)?.owners.remove(&word);
        for slot in &mut vm.stack {
            if slot
                .as_structural_root()
                .is_some_and(|key| key.get() == word)
            {
                *slot = Value::INVALID;
            }
        }
    }
    Ok(())
}

include!("cleanup/teardown.rs");
