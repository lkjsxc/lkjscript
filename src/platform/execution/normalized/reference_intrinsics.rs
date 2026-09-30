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
            "core.cell.create" => {
                let [scalar]: [CheckedValue; 1] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("cell create arity"))?;
                let NormalizedValue::I64(scalar) = scalar.release() else {
                    return Err(reference_type_error("cell create type"));
                };
                self.charge_allocation(
                    super::super::owned_i64_cell::OwnedI64Cell::ALLOCATION_BYTES,
                )?;
                self.control.check()?;
                CheckedValue::memory(
                    &self.schema,
                    NormalizedValue::OwnedI64Cell(super::super::owned_i64_cell::OwnedI64Cell::new(
                        self.memory_domain,
                        scalar,
                    )),
                )
            }
            "core.cell.replace" => {
                let [scalar, cell]: [CheckedValue; 2] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("cell replace arity"))?;
                let (NormalizedValue::I64(scalar), NormalizedValue::OwnedI64Cell(cell)) =
                    (scalar.release(), cell.release())
                else {
                    return Err(reference_type_error("cell replace types"));
                };
                cell.validate(self.memory_domain, true)?;
                CheckedValue::memory(
                    &self.schema,
                    NormalizedValue::OwnedI64Cell(cell.replace(scalar, self.control)?),
                )
            }
            "core.cell.read" => {
                let [cell]: [CheckedValue; 1] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("cell read arity"))?;
                let NormalizedValue::OwnedI64Cell(cell) = cell.raw() else {
                    return Err(reference_type_error("cell read type"));
                };
                cell.validate(self.memory_domain, false)?;
                if !cell.is_borrowed() {
                    return Err(reference_type_error("cell read requires a scoped loan"));
                }
                CheckedValue::primitive(&self.schema, NormalizedValue::I64(cell.read()?))
            }
            "core.cell.extract" | "core.cell.discard" => {
                let [cell]: [CheckedValue; 1] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("cell terminal arity"))?;
                let NormalizedValue::OwnedI64Cell(cell) = cell.release() else {
                    return Err(reference_type_error("cell terminal type"));
                };
                cell.validate(self.memory_domain, true)?;
                let scalar = cell.extract()?;
                CheckedValue::primitive(
                    &self.schema,
                    if implementation == "core.cell.extract" {
                        NormalizedValue::I64(scalar)
                    } else {
                        NormalizedValue::Unit
                    },
                )
            }
            "core.buffer.empty" => {
                if !arguments.is_empty() {
                    return Err(reference_type_error("buffer empty arity"));
                }
                let control = self.control;
                let buffer = super::super::byte_buffer::ByteBuffer::create(
                    self.memory_domain,
                    control,
                    &mut |bytes| self.charge_allocation(bytes),
                )?;
                CheckedValue::memory(&self.schema, NormalizedValue::ByteBuffer(buffer))
            }
            "core.buffer.push" => {
                let [octet, buffer]: [CheckedValue; 2] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("buffer push arity"))?;
                let (NormalizedValue::I64(octet), NormalizedValue::ByteBuffer(buffer)) =
                    (octet.release(), buffer.release())
                else {
                    return Err(reference_type_error("buffer push types"));
                };
                buffer.validate(self.memory_domain, true)?;
                let control = self.control;
                let buffer =
                    buffer.push(octet, control, &mut |bytes| self.charge_allocation(bytes))?;
                CheckedValue::memory(&self.schema, NormalizedValue::ByteBuffer(buffer))
            }
            "core.buffer.length" | "core.buffer.get" => {
                let expected = if implementation == "core.buffer.get" {
                    2
                } else {
                    1
                };
                if arguments.len() != expected {
                    return Err(reference_type_error("buffer read arity"));
                }
                let Some(NormalizedValue::ByteBuffer(buffer)) =
                    arguments.last().map(CheckedValue::raw)
                else {
                    return Err(reference_type_error("buffer read token"));
                };
                buffer.validate(self.memory_domain, false)?;
                if !buffer.is_borrowed() {
                    return Err(reference_type_error("buffer read requires a scoped loan"));
                }
                let value = if implementation == "core.buffer.get" {
                    let NormalizedValue::I64(index) = arguments[0].raw() else {
                        return Err(reference_type_error("buffer index type"));
                    };
                    buffer.get(*index)? as i64
                } else {
                    i64::try_from(buffer.len()?)
                        .map_err(|_| reference_type_error("buffer length overflow"))?
                };
                CheckedValue::primitive(&self.schema, NormalizedValue::I64(value))
            }
            "core.buffer.freeze" | "core.buffer.discard" => {
                let [buffer]: [CheckedValue; 1] = arguments
                    .try_into()
                    .map_err(|_| reference_type_error("buffer terminal arity"))?;
                let NormalizedValue::ByteBuffer(buffer) = buffer.release() else {
                    return Err(reference_type_error("buffer terminal type"));
                };
                buffer.validate(self.memory_domain, true)?;
                if implementation == "core.buffer.freeze" {
                    self.charge_allocation(std::mem::size_of::<Vec<u8>>() as u64)?;
                    self.control.check()?;
                    CheckedValue::primitive(&self.schema, NormalizedValue::Bytes(buffer.freeze()?))
                } else {
                    drop(buffer);
                    CheckedValue::primitive(&self.schema, NormalizedValue::Unit)
                }
            }
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
            "core.f64.parse-result" => {
                let [value] = arguments.as_slice() else {
                    return Err(reference_type_error("F64 parser requires one argument"));
                };
                let NormalizedValue::Text(text) = value.raw() else {
                    return Err(reference_type_error("F64 parser requires text"));
                };
                let number = Binary64::parse(text);
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
                            NormalizedValue::F64(
                                number.unwrap_or_else(|| Binary64::from_float(0.0)),
                            ),
                        )?,
                    ),
                ])
            }
            "core.f64.to-i64-result" => {
                let [value] = arguments.as_slice() else {
                    return Err(reference_type_error("F64 conversion requires one argument"));
                };
                let NormalizedValue::F64(number) = value.raw() else {
                    return Err(reference_type_error("F64 conversion requires F64"));
                };
                let converted = reference_f64_to_i64(number.to_float());
                self.result_record([
                    (
                        "valid",
                        CheckedValue::primitive(
                            &self.schema,
                            NormalizedValue::Bool(converted.is_some()),
                        )?,
                    ),
                    (
                        "value",
                        CheckedValue::primitive(
                            &self.schema,
                            NormalizedValue::I64(converted.unwrap_or_default()),
                        )?,
                    ),
                ])
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
            "core.bytes.slice" | "core.bytes.copy" => {
                let selected = match (implementation, arguments.as_slice()) {
                    ("core.bytes.slice", [value, start, end]) => {
                        match (value.raw(), start.raw(), end.raw()) {
                            (
                                NormalizedValue::Bytes(bytes),
                                NormalizedValue::I64(start),
                                NormalizedValue::I64(end),
                            ) => reference_byte_range(bytes, *start, *end)?,
                            _ => {
                                return Err(reference_type_error(
                                    "byte slicing received foreign values",
                                ));
                            }
                        }
                    }
                    ("core.bytes.copy", [value]) => match value.raw() {
                        NormalizedValue::Bytes(bytes) => bytes.as_ref(),
                        _ => {
                            return Err(reference_type_error(
                                "byte copying received a foreign value",
                            ));
                        }
                    },
                    _ => {
                        return Err(reference_type_error(
                            "byte range operation has foreign arity",
                        ));
                    }
                };
                // Deliberately materialize the range instead of sharing the optimized
                // carrier. Reserve scratch and retained storage at their own boundaries.
                self.charge_allocation(selected.len() as u64)?;
                let mut output = Vec::new();
                output.try_reserve_exact(selected.len()).map_err(|_| {
                    reference_resource("reference_bytes_storage", "cannot allocate byte range")
                })?;
                for chunk in selected.chunks(65_536) {
                    self.control.check()?;
                    output.extend_from_slice(chunk);
                }
                self.control.check()?;
                self.charge_allocation(output.len() as u64)?;
                CheckedValue::primitive(&self.schema, NormalizedValue::Bytes(output.into()))
            }
            "core.bytes.from-list" => {
                let [list] = arguments.as_slice() else {
                    return Err(reference_type_error("byte construction requires one list"));
                };
                let NormalizedValue::List(items) = list.raw() else {
                    return Err(reference_type_error(
                        "byte construction requires an integer list",
                    ));
                };
                self.charge_allocation(items.len() as u64)?;
                let mut output = vec![0_u8; items.len()];
                for (index, item) in items.iter().enumerate() {
                    self.control.check()?;
                    match item {
                        NormalizedValue::I64(value) if (0..=255).contains(value) => {
                            output[index] = *value as u8
                        }
                        NormalizedValue::I64(_) => {
                            return Err(reference_trap(
                                "reference_bytes_octet",
                                "byte value is outside 0 through 255",
                            ));
                        }
                        _ => {
                            return Err(reference_type_error(
                                "byte construction requires integer elements",
                            ));
                        }
                    }
                }
                self.control.check()?;
                self.charge_allocation(items.len() as u64)?;
                CheckedValue::primitive(&self.schema, NormalizedValue::Bytes(output.into()))
            }
            "core.bytes.to-text-result" => {
                let [source] = arguments.as_slice() else {
                    return Err(reference_type_error("UTF-8 decoder requires one argument"));
                };
                let NormalizedValue::Bytes(bytes) = source.raw() else {
                    return Err(reference_type_error("UTF-8 decoder requires bytes"));
                };
                let (valid, text) = match std::str::from_utf8(bytes) {
                    Ok(text) => (true, text),
                    Err(_) => (false, ""),
                };
                self.control.check()?;
                self.charge_allocation(text.len() as u64)?;
                self.result_record([
                    (
                        "valid",
                        CheckedValue::primitive(&self.schema, NormalizedValue::Bool(valid))?,
                    ),
                    (
                        "value",
                        CheckedValue::primitive(&self.schema, NormalizedValue::text(text))?,
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
                    super::super::codec::decode_typed_with_control(
                        self.schema.as_ref(),
                        bytes,
                        ty,
                        JsonLimits::default(),
                        self.control,
                    )
                } else {
                    super::super::data_codec_reference::decode_typed_with_control(
                        self.schema.as_ref(),
                        bytes,
                        ty,
                        self.control,
                    )
                };
                let (value, error) = match decoded {
                    Err(error)
                        if matches!(
                            error.class,
                            DiagnosticClass::Cancelled | DiagnosticClass::Resource
                        ) =>
                    {
                        return Err(reference_json_error(error));
                    }
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
                    self.control,
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
            self.charge_items(entries.len(), std::mem::size_of::<CheckedValue>())?;
            let mut output = Vec::with_capacity(entries.len());
            for key in entries.keys() {
                self.control.check()?;
                // The projected key shares its existing immutable allocation.
                let value = map
                    .lookup(key, &mut |charge| self.reserve_map(charge))?
                    .ok_or_else(|| reference_type_error("map entry disappeared"))?;
                let key = CheckedValue::primitive(&self.schema, key.to_value())?;
                output.push(self.map_entry_value(key, value)?);
            }
            return self.list_value(output);
        }
        let key = arguments
            .next()
            .ok_or_else(|| reference_type_error("map operation omits its key"))?;
        let key = self.map_key_value(key)?;
        let value = arguments.next();
        if arguments.next().is_some() {
            return Err(reference_type_error("map operation has foreign arity"));
        }
        match (operation, value) {
            ("core.map.contains", None) => CheckedValue::primitive(
                &self.schema,
                NormalizedValue::Bool(entries.contains_key(&key)),
            ),
            ("core.map.get", None) => map
                .lookup(&key, &mut |charge| self.reserve_map(charge))?
                .ok_or_else(|| {
                    reference_trap("reference_map_key_absent", "map lookup key is absent")
                }),
            ("core.map.get-or", Some(fallback)) => {
                match map.lookup(&key, &mut |charge| self.reserve_map(charge))? {
                    Some(value) => Ok(value),
                    None => Ok(fallback),
                }
            }
            ("core.map.insert", Some(value)) => self.update_map_value(map, key, Some(value)),
            ("core.map.remove", None) => self.update_map_value(map, key, None),
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
