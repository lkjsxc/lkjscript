//! Canonical-reference intrinsic results compose reference-owned proofs.

use super::*;

impl ReferenceState<'_> {
    fn result_record<const N: usize>(
        &mut self,
        fields: [(&str, CheckedValue); N],
    ) -> Result<CheckedValue, ExecutionError> {
        self.charge_items(N, std::mem::size_of::<(Name, NormalizedValue)>())?;
        let fields = fields
            .into_iter()
            .map(|(name, value)| {
                Name::new(name.to_owned())
                    .map(|name| (name, value))
                    .map_err(|_| reference_type_error("intrinsic field name is invalid"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.record_value(None, fields)
    }

    pub(super) fn checked_intrinsic(
        &mut self,
        signature: &ReferenceSignature,
        implementation: &str,
        types: &[TypeObjectDigest],
        arguments: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        self.control.check()?;
        match implementation {
            "identity_host" => {
                let mut values = arguments.into_iter();
                let value = values.next();
                if values.next().is_some() {
                    return Err(reference_type_error("identity host has foreign arity"));
                }
                match value {
                    Some(value) => Ok(value),
                    None => CheckedValue::primitive(&self.schema, NormalizedValue::Unit),
                }
            }
            "core.i64.parse-result" => {
                let [value] = arguments.as_slice() else {
                    return Err(reference_type_error("integer parser has foreign arity"));
                };
                let NormalizedValue::Text(text) = value.raw() else {
                    return Err(reference_type_error(
                        "integer parser received another scalar type",
                    ));
                };
                let number = reference_parse_i64(text);
                self.result_record([
                    (
                        "valid",
                        CheckedValue::primitive(
                            &self.schema,
                            NormalizedValue::Bool(number.is_some()),
                        )?,
                    ),
                    (
                        "value",
                        CheckedValue::primitive(
                            &self.schema,
                            NormalizedValue::I64(number.unwrap_or_default()),
                        )?,
                    ),
                ])
            }
            "core.list.get" => match arguments.as_slice() {
                [list, index] => match index.raw() {
                    NormalizedValue::I64(index) => {
                        list.at(usize::try_from(*index).map_err(|_| {
                            reference_trap(
                                "reference_list_index",
                                "list index is negative or excessive",
                            )
                        })?)
                    }
                    _ => Err(reference_type_error("list index is not an integer")),
                },
                _ => Err(reference_type_error("list lookup has foreign arity")),
            },
            "core.list.append" => {
                let [list, item]: [CheckedValue; 2] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("list append has foreign arity"))?;
                let NormalizedValue::List(items) = list.raw() else {
                    return Err(reference_type_error(
                        "list append received another value type",
                    ));
                };
                self.charge_items(
                    items.len().saturating_add(1),
                    std::mem::size_of::<NormalizedValue>(),
                )?;
                self.append_value(list, item)
            }
            "core.option.none" => {
                if !arguments.is_empty() {
                    return Err(reference_type_error("empty option has arguments"));
                }
                self.option_value(None)
            }
            "core.option.some" => {
                let [item]: [CheckedValue; 1] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("option constructor has foreign arity"))?;
                self.charge_items(1, std::mem::size_of::<NormalizedValue>())?;
                self.option_value(Some(item))
            }
            "core.option.get-or" => {
                let [option, fallback]: [CheckedValue; 2] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("option fallback has foreign arity"))?;
                match option.optional()? {
                    Some(value) => Ok(value),
                    None => Ok(fallback),
                }
            }
            "core.map.get" | "core.map.contains" | "core.map.get-or" | "core.map.insert"
            | "core.map.remove" | "core.map.entries" => {
                self.map_intrinsic(implementation, arguments)
            }
            "core.json.decode-or" | "core.data.decode-or" => {
                let [source, fallback]: [CheckedValue; 2] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("decoder has foreign arity"))?;
                let NormalizedValue::Bytes(bytes) = source.raw() else {
                    return Err(reference_type_error("decoder input is not bytes"));
                };
                let ty = match types {
                    [ty] => *ty,
                    [] if signature.type_parameters.is_empty() => signature
                        .parameters
                        .get(1)
                        .map(|parameter| parameter.ty)
                        .ok_or_else(|| reference_type_error("decoder fallback type is absent"))?,
                    _ => {
                        return Err(reference_type_error(
                            "decoder has no exact type substitution",
                        ));
                    }
                };
                let decoded = if implementation == "core.json.decode-or" {
                    decode_typed(self.schema.as_ref(), bytes, ty, JsonLimits::default())
                } else {
                    super::super::data_codec_reference::decode_typed(
                        self.schema.as_ref(),
                        bytes,
                        ty,
                    )
                };
                let (value, error) = match decoded {
                    Ok(value) => (
                        self.admit_raw(value, ty, &BTreeMap::new(), None, false)?,
                        None,
                    ),
                    Err(error) => (fallback, Some(error.code)),
                };
                if implementation == "core.data.decode-or" {
                    return Ok(value);
                }
                let valid = error.is_none();
                let error = error.unwrap_or_default();
                self.charge_allocation(error.len() as u64)?;
                self.result_record([
                    (
                        "error",
                        CheckedValue::primitive(&self.schema, NormalizedValue::text(error))?,
                    ),
                    (
                        "valid",
                        CheckedValue::primitive(&self.schema, NormalizedValue::Bool(valid))?,
                    ),
                    ("value", value),
                ])
            }
            _ => {
                let result = reference_intrinsic(
                    self.schema.as_ref(),
                    signature,
                    implementation,
                    types,
                    arguments.into_iter().map(CheckedValue::release).collect(),
                )?;
                let result = CheckedValue::primitive(&self.schema, result)?;
                self.charge_value(result.raw())?;
                Ok(result)
            }
        }
    }

    fn map_intrinsic(
        &mut self,
        operation: &str,
        arguments: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let mut arguments = arguments.into_iter();
        let map = arguments
            .next()
            .ok_or_else(|| reference_type_error("map operation omits its map"))?;
        let NormalizedValue::Map(entries) = map.raw() else {
            return Err(reference_type_error(
                "map operation received a foreign value",
            ));
        };
        if operation == "core.map.entries" {
            if arguments.next().is_some() {
                return Err(reference_type_error("map entries has foreign arity"));
            }
            self.charge_items(entries.len(), std::mem::size_of::<NormalizedValue>())?;
            let mut output = Vec::with_capacity(entries.len());
            for key in entries.keys() {
                self.control.check()?;
                let value = map
                    .lookup(key)?
                    .ok_or_else(|| reference_type_error("map entry disappeared"))?;
                output.push(self.result_record([
                    (
                        "key",
                        CheckedValue::primitive(&self.schema, key.to_value())?,
                    ),
                    ("value", value),
                ])?);
            }
            return self.list_value(output);
        }
        let key = arguments
            .next()
            .and_then(|key| NormalizedMapKey::from_value(key.release()))
            .ok_or_else(|| {
                reference_trap(
                    "reference_map_key",
                    "map operation requires an ordered primitive key",
                )
            })?;
        let value = arguments.next();
        if arguments.next().is_some() {
            return Err(reference_type_error("map operation has foreign arity"));
        }
        match (operation, value) {
            ("core.map.contains", None) => CheckedValue::primitive(
                &self.schema,
                NormalizedValue::Bool(entries.contains_key(&key)),
            ),
            ("core.map.get", None) => map.lookup(&key)?.ok_or_else(|| {
                reference_trap("reference_map_key_absent", "map lookup key is absent")
            }),
            ("core.map.get-or", Some(fallback)) => match map.lookup(&key)? {
                Some(value) => Ok(value),
                None => Ok(fallback),
            },
            ("core.map.insert", Some(value)) => {
                self.charge_items(
                    entries
                        .len()
                        .saturating_add(usize::from(!entries.contains_key(&key))),
                    std::mem::size_of::<(NormalizedMapKey, NormalizedValue)>(),
                )?;
                self.update_map_value(map, key, Some(value))
            }
            ("core.map.remove", None) => {
                self.charge_items(
                    entries
                        .len()
                        .saturating_sub(usize::from(entries.contains_key(&key))),
                    std::mem::size_of::<(NormalizedMapKey, NormalizedValue)>(),
                )?;
                self.update_map_value(map, key, None)
            }
            _ => Err(reference_type_error("map operation has foreign arity")),
        }
    }

    pub(super) fn validate_capability_arguments(
        &mut self,
        requirement: RequirementReference,
        parameters: &[ParameterRecord],
        arguments: &[CheckedValue],
    ) -> Result<(), ExecutionError> {
        if parameters.len() != arguments.len() {
            return Err(reference_type_error(
                "capability argument count disagrees with canonical parameters",
            ));
        }
        for (parameter, argument) in parameters.iter().zip(arguments) {
            let ownership = argument.ownership(&self.schema, &mut self.observation.value_work)?;
            match parameter.use_mode {
                ParameterUse::Unrestricted if ownership == Ownership::Ordinary => {}
                ParameterUse::Borrow | ParameterUse::Consume
                    if ownership == Ownership::Capability =>
                {
                    let NormalizedValue::Resource(handle) = argument.raw() else {
                        return Err(reference_type_error(
                            "capability operation requires a direct owner",
                        ));
                    };
                    let Some(OwnerRecord::Requirement(record)) = self.owner_in_package(
                        requirement.package,
                        OwnerKey::Requirement(requirement.requirement),
                    )?
                    else {
                        return Err(reference_type_error(
                            "capability requirement is absent from canonical authority",
                        ));
                    };
                    self.resources.validate_admission(
                        *handle,
                        Some(requirement),
                        Some(record.interface),
                    )?;
                }
                _ => {
                    return Err(reference_type_error(
                        "capability ownership disagrees with its exact canonical parameter use",
                    ));
                }
            }
        }
        Ok(())
    }
}
