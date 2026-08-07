use std::num::NonZeroU64;

use lkjscript_core::{
    Constant, LayoutIdentity, OwnedValue, SemanticPayload, SemanticTypeIdentity, SemanticValue,
    StructuralKind, StructuralNodeView, StructuralType,
};

use super::*;

fn host_type(kind: StructuralKind) -> StructuralType {
    let identity = match kind {
        StructuralKind::String => NonZeroU64::MIN,
        StructuralKind::Path => NonZeroU64::MAX,
        _ => unreachable!("host structural leaf kind"),
    };
    StructuralType::new(
        LayoutIdentity::new(identity),
        SemanticTypeIdentity::new(identity),
        kind,
    )
}

pub(in crate::run) fn publish_string(vm: &mut Vm<'_>, text: String) -> Result<Value> {
    publish_host_leaf(
        vm,
        SemanticValue::new(
            host_type(StructuralKind::String),
            SemanticPayload::String(text.into_bytes()),
        ),
    )
}

fn publish_host_leaf(vm: &mut Vm<'_>, semantic: SemanticValue) -> Result<Value> {
    let value_type = semantic.value_type;
    let key = invocation_mut(vm)?
        .runtime
        .publish_owned(semantic)
        .map_err(|failure| map_value_error(failure.error))?;
    invocation_mut(vm)?.register_host_owner(key, value_type)
}

pub(in crate::run) fn semantic_snapshot(vm: &Vm<'_>, value: Value) -> Result<SemanticValue> {
    let key = value
        .as_structural_root()
        .ok_or_else(|| Error::msg("value is not a structural owner"))?;
    let structural = invocation(vm)?;
    let value_type = registered_owner_type(structural, key)
        .ok_or_else(|| Error::msg("structural owner is stale or unregistered"))?;
    structural
        .runtime
        .value(key, value_type)
        .map_err(map_value_error)
}

pub(in crate::run) fn copy_string(vm: &Vm<'_>, value: Value) -> Result<String> {
    if let Some(index) = value.as_static_string() {
        return match vm.chunk.constant(index) {
            Some(Constant::Str(text)) => Ok(text.clone()),
            _ => Err(Error::msg("stale static string constant")),
        };
    }
    let (key, value_type) = leaf_owner(vm, value, StructuralKind::String)?;
    let node = invocation(vm)?
        .runtime
        .value_node(key, value_type)
        .map_err(map_value_error)?;
    let StructuralNodeView::Bytes(bytes) = node.payload() else {
        return Err(Error::msg("structural string owner has the wrong payload"));
    };
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| Error::msg("structural string payload is not UTF-8"))
}

pub(in crate::run) fn copy_path(vm: &Vm<'_>, value: Value) -> Result<Vec<u8>> {
    let (key, value_type) = leaf_owner(vm, value, StructuralKind::Path)?;
    let node = invocation(vm)?
        .runtime
        .value_node(key, value_type)
        .map_err(map_value_error)?;
    let StructuralNodeView::Bytes(bytes) = node.payload() else {
        return Err(Error::msg("structural path owner has the wrong payload"));
    };
    Ok(bytes.to_vec())
}

fn leaf_owner(
    vm: &Vm<'_>,
    value: Value,
    expected: StructuralKind,
) -> Result<(StructuralValueKey, StructuralType)> {
    let key = value
        .as_structural_root()
        .ok_or_else(|| Error::msg("expected exact structural leaf owner"))?;
    let structural = invocation(vm)?;
    let value_type = registered_owner_type(structural, key)
        .ok_or_else(|| Error::msg("stale, forged, or unregistered structural leaf owner"))?;
    if value_type.kind != expected {
        return Err(Error::msg(
            "structural leaf owner has the wrong runtime kind",
        ));
    }
    Ok((key, value_type))
}

pub(in crate::run) fn static_string_semantic(vm: &Vm<'_>, value: Value) -> Result<SemanticValue> {
    let index = value
        .as_static_string()
        .ok_or_else(|| Error::msg("expected static string artifact"))?;
    let text = match vm.chunk.constant(index) {
        Some(Constant::Str(text)) => text,
        _ => return Err(Error::msg("stale static string artifact")),
    };
    Ok(SemanticValue::new(
        host_type(StructuralKind::String),
        SemanticPayload::String(text.as_bytes().to_vec()),
    ))
}

pub(in crate::run) fn export_plain_return(
    vm: &mut Vm<'_>,
    value: Value,
) -> Result<Option<OwnedValue>> {
    if let Some(index) = value.as_static_string() {
        let bytes = match vm.chunk.constant(index) {
            Some(Constant::Str(text)) => text.len(),
            _ => return Err(Error::msg("stale static string constant")),
        };
        vm.preflight_allocation(1)?;
        vm.preflight_heap_growth(
            u64::try_from(bytes).map_err(|_| Error::host("static string length exceeds u64"))?,
        )?;
        vm.preflight_output(bytes)?;
        let owned = OwnedValue::from_structural(static_string_semantic(vm, value)?)?;
        vm.record_output(bytes)?;
        return Ok(Some(owned));
    }
    let Some(key) = value.as_structural_root() else {
        return Ok(None);
    };
    let (value_type, host_owner) =
        if let Some(value_type) = invocation(vm)?.host_owners.get(&key.get()).copied() {
            (value_type, true)
        } else if let Some(record) = invocation(vm)?.owners.get(&key.get()).copied() {
            (record.value_type, false)
        } else {
            return Ok(None);
        };
    let accounting = preflight_owned_export(vm, key, value_type, host_owner)?;
    let semantic = invocation_mut(vm)?
        .runtime
        .export_semantic(key, value_type)
        .map_err(map_value_error)?;
    if host_owner {
        invocation_mut(vm)?.host_owners.remove(&key.get());
    } else {
        invocation_mut(vm)?.owners.remove(&key.get());
    }
    let owned = OwnedValue::from_structural(semantic)?;
    commit_export_output(vm, accounting)?;
    Ok(Some(owned))
}

pub(super) fn is_host_owner(invocation: &StructuralInvocation, value: Value) -> bool {
    value
        .as_structural_root()
        .is_some_and(|key| invocation.host_owners.contains_key(&key.get()))
}

pub(super) fn drop_host_owner(vm: &mut Vm<'_>, value: Value) -> Result<()> {
    let key = value
        .as_structural_root()
        .ok_or_else(|| Error::msg("host structural owner changed category"))?;
    let value_type = invocation(vm)?
        .host_owners
        .get(&key.get())
        .copied()
        .ok_or_else(|| Error::msg("host structural owner disappeared"))?;
    invocation_mut(vm)?
        .runtime
        .drop_owned(key, value_type)
        .map_err(map_value_error)?;
    invocation_mut(vm)?.host_owners.remove(&key.get());
    Ok(())
}
