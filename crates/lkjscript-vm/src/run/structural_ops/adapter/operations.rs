fn require_resource_result_metadata(chunk: &ValidatedChunk) -> Result<()> {
    let definition = chunk
        .enums()
        .iter()
        .find(|item| item.id.bytes() == lkjscript_core::RESULT_ID)
        .ok_or_else(|| Error::msg("resource Result enum metadata is missing"))?;
    if definition.layout.bytes() != lkjscript_core::RESULT_LAYOUT {
        return Err(Error::msg("resource Result layout identity mismatch"));
    }
    Ok(())
}

pub(in crate::run) fn adapter_is_variant<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    value: Value,
    enum_id: EnumId,
    layout: RuntimeLayoutId,
    variant: VariantId,
) -> Result<Option<bool>> {
    if value.as_aggregate_adapter().is_none() {
        return Ok(None);
    }
    let record = invocation(vm)?.adapters.get(value)?;
    if record.enum_id != enum_id || record.layout != layout {
        return Err(Error::msg(
            "aggregate adapter enum identity/layout mismatch",
        ));
    }
    Ok(Some(record.variant == variant))
}

pub(in crate::run) fn adapter_take_field<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    value: Value,
    enum_id: EnumId,
    layout: RuntimeLayoutId,
    variant: VariantId,
) -> Result<Option<Value>> {
    if value.as_aggregate_adapter().is_none() {
        return Ok(None);
    }
    let record = invocation_mut(vm)?.adapters.take(value)?;
    if record.enum_id != enum_id || record.layout != layout || record.variant != variant {
        cleanup_adapter_payload(vm, record.payload, None)?;
        return Err(Error::msg(
            "aggregate adapter projection identity/layout/variant mismatch",
        ));
    }
    Ok(Some(match record.payload {
        AdapterPayload::Resource { value, .. } | AdapterPayload::Structural(value) => value,
    }))
}

pub(in crate::run) fn drop_resource_adapter<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    value: Value,
    expected: ResourceKind,
) -> Option<Result<()>> {
    value.as_aggregate_adapter()?;
    Some(
        invocation_mut(vm)
            .and_then(|invocation| invocation.adapters.take(value))
            .and_then(|record| cleanup_adapter_payload(vm, record.payload, Some(expected))),
    )
}

pub(super) fn cleanup_all_adapters<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let values = invocation(vm)?.adapters.live_values();
    for value in values {
        let record = invocation_mut(vm)?.adapters.take(value)?;
        cleanup_adapter_payload(vm, record.payload, None)?;
    }
    Ok(())
}

fn cleanup_adapter_payload<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    payload: AdapterPayload,
    expected: Option<ResourceKind>,
) -> Result<()> {
    match payload {
        AdapterPayload::Structural(value) => drop_registered_owner(vm, value),
        AdapterPayload::Resource { value, kind } => {
            if expected.is_some_and(|expected| expected != kind) {
                return Err(Error::msg("resource aggregate cleanup kind mismatch"));
            }
            match kind {
                ResourceKind::SqliteConnection => vm.resources.sqlite_close(value).map(|_| ()),
                ResourceKind::SqliteStatement => vm.resources.sqlite_finalize(value).map(|_| ()),
                _ => vm.resources.close(value).map(|_| ()),
            }
        }
    }
}

pub(in crate::run) fn drop_registered_owner<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    value: Value,
) -> Result<()> {
    let (key, record) = invocation(vm)?.owner(value)?;
    invocation_mut(vm)?
        .runtime
        .dispose_owner(key, record.value_type)
        .map(|_| ())
        .map_err(map_value_error)?;
    invocation_mut(vm)?.owners.remove(&key.get());
    Ok(())
}
