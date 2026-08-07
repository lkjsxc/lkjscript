pub(in crate::run) fn publish_option(
    vm: &mut Vm<'_>,
    element: HostValueType,
    value: Option<HostValue>,
) -> Result<Value> {
    if matches!(element, HostValueType::Resource(_)) {
        return Err(Error::msg(
            "resource-bearing option has no exact nontraced VM adapter",
        ));
    }
    publish_deterministic(vm, HostValue::option(element, value))
}

pub(in crate::run) fn publish_numeric_result(
    vm: &mut Vm<'_>,
    success: HostValueType,
    result: std::result::Result<HostValue, lkjscript_core::NumericError>,
) -> Result<Value> {
    publish_deterministic(
        vm,
        HostValue::Result {
            ok: success,
            error: HostValueType::NumericError,
            value: match result {
                Ok(value) => Ok(Box::new(value)),
                Err(error) => Err(Box::new(HostValue::NumericError(error))),
            },
        },
    )
}

pub(in crate::run) fn publish_utf8_result(
    vm: &mut Vm<'_>,
    result: std::result::Result<HostValue, Utf8Failure>,
) -> Result<Value> {
    let value = HostValue::Result {
        ok: HostValueType::String,
        error: HostValueType::Utf8Error,
        value: match result {
            Ok(value) => Ok(Box::new(value)),
            Err(error) => Err(Box::new(HostValue::Utf8Error(error))),
        },
    };
    publish_deterministic(vm, value)
}

pub(in crate::run) fn publish_system_result(
    vm: &mut Vm<'_>,
    success: HostValueType,
    kind: SystemErrorKind,
    result: std::result::Result<HostValue, Error>,
) -> Result<Value> {
    let result = match result {
        Err(error) if error.class() != ErrorClass::Ordinary => return Err(error),
        result => result,
    };
    let resource = match success {
        HostValueType::Resource(kind) => Some(kind),
        _ => None,
    };
    if let Some(resource_kind) = resource {
        return publish_resource_result(vm, resource_kind, kind, result);
    }
    let value = HostValue::Result {
        ok: success,
        error: HostValueType::SystemError,
        value: match result {
            Ok(value) => Ok(Box::new(value)),
            Err(error) => Err(Box::new(HostValue::SystemError {
                kind,
                detail: error.to_string(),
            })),
        },
    };
    publish_deterministic(vm, value)
}

pub(in crate::run) fn publish_system_utf8_result(
    vm: &mut Vm<'_>,
    error: Utf8Failure,
) -> Result<Value> {
    publish_deterministic(
        vm,
        HostValue::Result {
            ok: HostValueType::String,
            error: HostValueType::SystemError,
            value: Err(Box::new(HostValue::SystemUtf8(error))),
        },
    )
}

fn publish_resource_result(
    vm: &mut Vm<'_>,
    resource_kind: ResourceKind,
    error_kind: SystemErrorKind,
    result: std::result::Result<HostValue, Error>,
) -> Result<Value> {
    require_resource_result_metadata(vm.chunk)?;
    let (variant, physical_tag, payload) = match result {
        Ok(HostValue::Resource { kind, value }) if kind == resource_kind => (
            VariantId::new(lkjscript_core::RESULT_OK_ID),
            0,
            AdapterPayload::Resource { value, kind },
        ),
        Ok(_) => return Err(Error::msg("resource result success payload shape mismatch")),
        Err(error) => {
            let structural = publish_deterministic(
                vm,
                HostValue::SystemError {
                    kind: error_kind,
                    detail: error.to_string(),
                },
            )?;
            (
                VariantId::new(lkjscript_core::RESULT_ERR_ID),
                1,
                AdapterPayload::Structural(structural),
            )
        }
    };
    let record = AdapterRecord {
        enum_id: EnumId::new(lkjscript_core::RESULT_ID),
        layout: RuntimeLayoutId::new(lkjscript_core::RESULT_LAYOUT),
        variant,
        physical_tag,
        payload,
    };
    match invocation_mut(vm)?.adapters.allocate(record) {
        Ok(value) => Ok(value),
        Err(error) => {
            if let AdapterPayload::Structural(value) = payload {
                let _ = drop_registered_owner(vm, value);
            }
            Err(error)
        }
    }
}

fn publish_deterministic(vm: &mut Vm<'_>, value: HostValue) -> Result<Value> {
    let value_type = declared_type(&value);
    let type_id = exact_structural_type(vm.chunk, &value_type)?;
    let semantic = semantic_for_type(vm.chunk, type_id, value)?;
    let runtime_type = semantic.value_type;
    let key = invocation_mut(vm)?
        .runtime
        .publish_owned(semantic)
        .map_err(|failure| map_value_error(failure.error))?;
    let representation = exact_owner_representation(vm.chunk, type_id)?;
    match invocation_mut(vm)?.register_owner(key, representation, runtime_type) {
        Ok(value) => Ok(value),
        Err(error) => {
            let _ = invocation_mut(vm)?.runtime.drop_owned(key, runtime_type);
            Err(error)
        }
    }
}
