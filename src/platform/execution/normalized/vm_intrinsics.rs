//! Closed intrinsic propagation; arbitrary hosts enter through raw result admission.

use super::*;

impl Machine<'_> {
    pub(super) fn charge_map(
        &mut self,
        charge: super::super::map::Charge,
    ) -> Result<(), ExecutionError> {
        self.control.check()?;
        reserve_execution_storage(
            self.policy,
            self.shared_budget.as_deref(),
            &mut self.observation,
            charge.bytes,
            charge.slots,
        )
    }

    pub(super) fn checked_map_key(
        &mut self,
        key: CheckedValue,
    ) -> Result<NormalizedMapKey, ExecutionError> {
        // Moving the immutable key handle allocates no payload; keep cancellation.
        self.charge_map(super::super::map::Charge::default())?;
        let key = NormalizedMapKey::from_value(key.into_raw()).ok_or_else(|| {
            trap_error("normalized_map_key", "map key is not an ordered primitive")
        })?;
        Ok(key)
    }

    pub(super) fn edit_checked_map(
        &mut self,
        map: CheckedValue,
        key: NormalizedMapKey,
        child: Option<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let mut work = std::mem::take(&mut self.observation.value_work);
        let program = self.program;
        let result = map.edit_map(program, key, child, &mut work, &mut |charge| {
            self.charge_map(charge)
        });
        self.observation.value_work = work;
        result
    }

    fn charge_list(&mut self, charge: super::super::list::Charge) -> Result<(), ExecutionError> {
        self.control.check()?;
        reserve_execution_storage(
            self.policy,
            self.shared_budget.as_deref(),
            &mut self.observation,
            charge.bytes,
            charge.slots,
        )
    }

    pub(super) fn construct_list(
        &mut self,
        items: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let mut work = std::mem::take(&mut self.observation.value_work);
        let program = self.program;
        let maximum = self
            .policy
            .maximum_collection_items
            .unwrap_or(super::super::list::MAXIMUM_LENGTH as u64);
        let result = CheckedValue::list(program, items, &mut work, maximum, &mut |charge| {
            self.charge_list(charge)
        });
        self.observation.value_work = work;
        result
    }

    pub(super) fn admit(
        &mut self,
        raw: NormalizedValue,
        ty: TypeObjectDigest,
        authority: Option<crate::platform::kernel::RequirementReference>,
        input: bool,
    ) -> Result<CheckedValue, ExecutionError> {
        self.admit_instantiated(raw, ty, authority, input, &BTreeMap::new())
    }

    pub(super) fn admit_instantiated(
        &mut self,
        raw: NormalizedValue,
        ty: TypeObjectDigest,
        authority: Option<crate::platform::kernel::RequirementReference>,
        input: bool,
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> Result<CheckedValue, ExecutionError> {
        checked::Admission {
            shared_budget: self.shared_budget.as_deref(),
            substitutions,
            program: self.program,
            resources: self.resources,
            control: self.control,
            policy: self.policy,
            admission_bytes: 0,
            admission_bytes_saturated: false,
            admission_items: 0,
            work: &mut self.observation.value_work,
            allocated: &mut self.observation.allocated_bytes,
            allocated_saturated: &mut self.observation.allocated_bytes_saturated,
            allocation_charges: &mut self.observation.allocation_charges,
            items: &mut self.observation.collection_items,
            items_saturated: &mut self.observation.collection_items_saturated,
        }
        .value(raw, ty, authority, input)
    }

    pub(super) fn admit_arguments(
        &mut self,
        index: FunctionIndex,
        types: &[TypeObjectDigest],
        raw: Vec<NormalizedValue>,
    ) -> Result<Vec<CheckedValue>, ExecutionError> {
        let raw = super::super::value::RawArguments::new(raw);
        let function = self
            .program
            .functions
            .get(index.0 as usize)
            .filter(|_| index.1 == self.program.value_origin)
            .ok_or_else(|| type_error("raw invocation selects a foreign prepared function"))?;
        if raw.len() != function.parameters.len() || types.len() != function.type_parameters.len() {
            return Err(type_error(
                "raw invocation has a foreign argument or type arity",
            ));
        }
        for (constraint, ty) in function.type_parameter_constraints.iter().zip(types) {
            self.control.check()?;
            resolve_runtime_type(
                self.program,
                *ty,
                &BTreeMap::new(),
                self.control,
                "raw invocation requires fully resolved exact types",
            )?;
            if constraint.requires_transfer() {
                transfer::admit_type(self.program, *ty, self.control)?;
            }
            if constraint.requires_share() {
                transfer::admit_shareable_type(self.program, *ty, self.control)?;
            }
            if constraint.requires_capture_safe() {
                checked::Admission {
                    shared_budget: self.shared_budget.as_deref(),
                    substitutions: &BTreeMap::new(),
                    program: self.program,
                    resources: self.resources,
                    control: self.control,
                    policy: self.policy,
                    admission_bytes: 0,
                    admission_bytes_saturated: false,
                    admission_items: 0,
                    work: &mut self.observation.value_work,
                    allocated: &mut self.observation.allocated_bytes,
                    allocated_saturated: &mut self.observation.allocated_bytes_saturated,
                    allocation_charges: &mut self.observation.allocation_charges,
                    items: &mut self.observation.collection_items,
                    items_saturated: &mut self.observation.collection_items_saturated,
                }
                .require_capture_type(
                    *ty,
                    &BTreeMap::new(),
                    &mut std::collections::BTreeSet::new(),
                )?;
            }
        }
        self.charge_allocation(
            (types.len() * std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()) as u64,
        )?;
        let substitutions = function
            .type_parameters
            .iter()
            .copied()
            .zip(types.iter().copied())
            .collect();
        let parameters = function.parameters.clone();
        raw.into_iter()
            .zip(parameters.iter())
            .map(|(value, parameter)| {
                let authority = parameter
                    .resource_requirement
                    .map(|index| self.program.requirements[index.0 as usize].reference);
                self.admit_instantiated(value, parameter.ty, authority, true, &substitutions)
            })
            .collect()
    }

    pub(super) fn admit_port_arguments(
        &mut self,
        ty: TypeObjectDigest,
        raw: Vec<NormalizedValue>,
    ) -> Result<Vec<CheckedValue>, ExecutionError> {
        let raw = super::super::value::RawArguments::new(raw);
        let parameters = match self.program.types.get(&ty).map(|ty| &ty.form) {
            Some(
                TypeForm::Function { parameters, .. } | TypeForm::TaskFunction { parameters, .. },
            ) => parameters.clone(),
            _ => {
                return Err(type_error(
                    "expression-backed port has no exact callable type",
                ));
            }
        };
        if parameters.len() != raw.len() {
            return Err(type_error("raw port invocation has a foreign arity"));
        }
        raw.into_iter()
            .zip(parameters)
            .map(|(value, ty)| self.admit(value, ty, None, true))
            .collect()
    }

    pub(super) fn validate_operation_arguments(
        &mut self,
        requirement: RequirementIndex,
        operation: super::super::value::OperationIndex,
        arguments: &[CheckedValue],
    ) -> Result<(), ExecutionError> {
        let operation = self
            .program
            .operations
            .get(operation.0 as usize)
            .ok_or_else(|| type_error("capability operation is absent"))?;
        let requirement = self
            .program
            .requirements
            .get(requirement.0 as usize)
            .ok_or_else(|| type_error("capability requirement is absent"))?;
        if operation.parameters.len() != arguments.len() {
            return Err(type_error("capability operation has a foreign arity"));
        }
        for (parameter, argument) in operation.parameters.iter().zip(arguments) {
            let class = argument.class(self.program, &mut self.observation.value_work)?;
            match parameter.use_mode {
                ParameterUse::Unrestricted if class == Class::Free => {}
                ParameterUse::Borrow | ParameterUse::Consume if class == Class::Direct => {
                    let NormalizedValue::Resource(handle) = argument.raw() else {
                        return Err(type_error("capability parameter is not a direct owner"));
                    };
                    self.resources.validate_admission(
                        *handle,
                        Some(requirement.reference),
                        Some(requirement.interface),
                    )?;
                }
                _ => {
                    return Err(type_error(
                        "capability parameter use disagrees with checked ownership",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) fn push_scalar(&mut self, raw: NormalizedValue) -> Result<(), ExecutionError> {
        self.push(CheckedValue::scalar(self.program, raw)?)
    }

    pub(super) fn intrinsic_record<const N: usize>(
        &mut self,
        fields: [(&str, CheckedValue); N],
    ) -> Result<CheckedValue, ExecutionError> {
        self.charge_collection(N, std::mem::size_of::<(Name, NormalizedValue)>())?;
        let fields = fields
            .into_iter()
            .map(|(name, value)| {
                Name::new(name.to_owned())
                    .map(|name| (name, value))
                    .map_err(|_| {
                        runtime_error(
                            "normalized_intrinsic_field",
                            "intrinsic field name is invalid",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        CheckedValue::record(self.program, None, fields, &mut self.observation.value_work)
    }

    pub(super) fn call_checked_intrinsic(
        &mut self,
        function: &super::super::prepare::NormalizedFunction,
        implementation: &str,
        types: &[TypeObjectDigest],
        arguments: &mut Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        self.control.check()?;
        match implementation {
            "core.cell.create" => {
                let [scalar] = take_intrinsic_arguments::<1>(arguments, "cell create arity")?;
                let NormalizedValue::I64(scalar) = scalar.into_raw() else {
                    return Err(type_error("cell create type"));
                };
                self.charge_allocation(
                    super::super::owned_i64_cell::OwnedI64Cell::ALLOCATION_BYTES,
                )?;
                self.control.check()?;
                let cell =
                    super::super::owned_i64_cell::OwnedI64Cell::new(self.memory_domain, scalar);
                cell.establish_admission(self.program.value_origin)?;
                CheckedValue::memory(self.program, NormalizedValue::OwnedI64Cell(cell))
            }
            "core.cell.replace" => {
                let [scalar, cell] =
                    take_intrinsic_arguments::<2>(arguments, "cell replace arity")?;
                let (NormalizedValue::I64(scalar), NormalizedValue::OwnedI64Cell(cell)) =
                    (scalar.into_raw(), cell.into_raw())
                else {
                    return Err(type_error("cell replace types"));
                };
                cell.validate(self.memory_domain, true)?;
                let cell = cell.replace(scalar, self.control)?;
                cell.establish_admission(self.program.value_origin)?;
                CheckedValue::memory(self.program, NormalizedValue::OwnedI64Cell(cell))
            }
            "core.cell.read" => {
                let [cell] = take_intrinsic_arguments::<1>(arguments, "cell read arity")?;
                let NormalizedValue::OwnedI64Cell(cell) = cell.raw() else {
                    return Err(type_error("cell read type"));
                };
                cell.validate(self.memory_domain, false)?;
                if !cell.is_borrowed() {
                    return Err(type_error("cell read requires a scoped loan"));
                }
                CheckedValue::scalar(self.program, NormalizedValue::I64(cell.read()?))
            }
            "core.cell.extract" | "core.cell.discard" => {
                let [cell] = take_intrinsic_arguments::<1>(arguments, "cell terminal arity")?;
                let NormalizedValue::OwnedI64Cell(cell) = cell.into_raw() else {
                    return Err(type_error("cell terminal type"));
                };
                cell.validate(self.memory_domain, true)?;
                let scalar = cell.extract()?;
                CheckedValue::scalar(
                    self.program,
                    if implementation == "core.cell.extract" {
                        NormalizedValue::I64(scalar)
                    } else {
                        NormalizedValue::Unit
                    },
                )
            }
            "core.buffer.empty" => {
                if !arguments.is_empty() {
                    return Err(type_error("buffer empty arity"));
                }
                let control = self.control;
                let buffer = super::super::byte_buffer::ByteBuffer::create(
                    self.memory_domain,
                    control,
                    &mut |bytes| self.charge_allocation(bytes),
                )?;
                buffer.establish_admission(self.program.value_origin)?;
                CheckedValue::memory(self.program, NormalizedValue::ByteBuffer(buffer))
            }
            "core.buffer.push" => {
                let [octet, buffer] =
                    take_intrinsic_arguments::<2>(arguments, "buffer push arity")?;
                let (NormalizedValue::I64(octet), NormalizedValue::ByteBuffer(buffer)) =
                    (octet.into_raw(), buffer.into_raw())
                else {
                    return Err(type_error("buffer push types"));
                };
                buffer.validate(self.memory_domain, true)?;
                let control = self.control;
                let buffer =
                    buffer.push(octet, control, &mut |bytes| self.charge_allocation(bytes))?;
                buffer.establish_admission(self.program.value_origin)?;
                CheckedValue::memory(self.program, NormalizedValue::ByteBuffer(buffer))
            }
            "core.buffer.length" | "core.buffer.get" => {
                let expected = if implementation == "core.buffer.get" {
                    2
                } else {
                    1
                };
                if arguments.len() != expected {
                    return Err(type_error("buffer read arity"));
                }
                let Some(NormalizedValue::ByteBuffer(buffer)) =
                    arguments.last().map(CheckedValue::raw)
                else {
                    return Err(type_error("buffer read token"));
                };
                buffer.validate(self.memory_domain, false)?;
                if !buffer.is_borrowed() {
                    return Err(type_error("buffer read requires a scoped loan"));
                }
                let value = if implementation == "core.buffer.get" {
                    let NormalizedValue::I64(index) = arguments[0].raw() else {
                        return Err(type_error("buffer index type"));
                    };
                    buffer.get(*index)? as i64
                } else {
                    i64::try_from(buffer.len()?)
                        .map_err(|_| type_error("buffer length overflow"))?
                };
                CheckedValue::scalar(self.program, NormalizedValue::I64(value))
            }
            "core.buffer.freeze" | "core.buffer.discard" => {
                let [buffer] = take_intrinsic_arguments::<1>(arguments, "buffer terminal arity")?;
                let NormalizedValue::ByteBuffer(buffer) = buffer.into_raw() else {
                    return Err(type_error("buffer terminal type"));
                };
                buffer.validate(self.memory_domain, true)?;
                if implementation == "core.buffer.freeze" {
                    self.charge_allocation(std::mem::size_of::<Vec<u8>>() as u64)?;
                    self.control.check()?;
                    CheckedValue::scalar(self.program, NormalizedValue::Bytes(buffer.freeze()?))
                } else {
                    drop(buffer);
                    CheckedValue::scalar(self.program, NormalizedValue::Unit)
                }
            }
            "identity_host" => match arguments.len() {
                0 => CheckedValue::scalar(self.program, NormalizedValue::Unit),
                1 => arguments
                    .pop()
                    .ok_or_else(|| type_error("identity arity changed")),
                _ => Err(type_error("identity host received a foreign arity")),
            },
            "core.bytes.concat" => {
                let [left, right] = take_intrinsic_arguments::<2>(
                    arguments,
                    "byte concatenation received a foreign arity",
                )?;
                let (NormalizedValue::Bytes(left), NormalizedValue::Bytes(right)) =
                    (left.into_raw(), right.into_raw())
                else {
                    return Err(type_error("byte concatenation received foreign values"));
                };
                let control = self.control;
                let mut work = std::mem::take(&mut self.observation.value_work.bytes);
                let result = left.concat(
                    right,
                    None,
                    control,
                    &mut |charge| self.charge_allocation(charge),
                    &mut work,
                );
                // Failure preserves the actual attempted/copy work, not a fabricated success.
                self.observation.value_work.bytes = work;
                CheckedValue::scalar(self.program, NormalizedValue::Bytes(result?))
            }
            "core.bytes.slice" => {
                let [value, start, end] = take_intrinsic_arguments::<3>(
                    arguments,
                    "byte slicing received a foreign arity",
                )?;
                let (
                    NormalizedValue::Bytes(value),
                    NormalizedValue::I64(start),
                    NormalizedValue::I64(end),
                ) = (value.into_raw(), start.into_raw(), end.into_raw())
                else {
                    return Err(type_error("byte slicing received foreign values"));
                };
                let control = self.control;
                let result = value.slice(start, end, control, &mut |bytes| {
                    self.charge_allocation(bytes)
                })?;
                CheckedValue::scalar(self.program, NormalizedValue::Bytes(result))
            }
            "core.bytes.copy" => {
                let [value] = take_intrinsic_arguments::<1>(
                    arguments,
                    "byte copying received a foreign arity",
                )?;
                let NormalizedValue::Bytes(value) = value.into_raw() else {
                    return Err(type_error("byte copying received a foreign value"));
                };
                let control = self.control;
                let result =
                    value.detached_copy(control, &mut |bytes| self.charge_allocation(bytes))?;
                CheckedValue::scalar(self.program, NormalizedValue::Bytes(result))
            }
            "core.bytes.from-list" => {
                let [value] = arguments.as_slice() else {
                    return Err(type_error("byte construction received a foreign arity"));
                };
                let NormalizedValue::List(items) = value.raw() else {
                    return Err(type_error("byte construction requires an integer list"));
                };
                // Reserve the result before allocation. Never clamp or wrap an octet.
                self.charge_allocation(items.len() as u64)?;
                let mut bytes = Vec::with_capacity(items.len());
                for item in items.iter() {
                    self.control.check()?;
                    let NormalizedValue::I64(value) = item else {
                        return Err(type_error("byte construction requires integer elements"));
                    };
                    bytes.push(u8::try_from(*value).map_err(|_| {
                        trap_error(
                            "normalized_bytes_octet",
                            "byte value is outside 0 through 255",
                        )
                    })?);
                }
                self.control.check()?;
                self.charge_allocation(items.len() as u64)?;
                CheckedValue::scalar(self.program, NormalizedValue::Bytes(bytes.into()))
            }
            "core.bytes.to-text-result" => {
                let [value] = arguments.as_slice() else {
                    return Err(type_error("UTF-8 decoding received a foreign arity"));
                };
                let NormalizedValue::Bytes(bytes) = value.raw() else {
                    return Err(type_error("UTF-8 decoding requires bytes"));
                };
                let decoded = std::str::from_utf8(bytes);
                self.control.check()?;
                let text = decoded.unwrap_or_default();
                self.charge_allocation(text.len() as u64)?;
                self.intrinsic_record([
                    (
                        "valid",
                        CheckedValue::scalar(self.program, NormalizedValue::Bool(decoded.is_ok()))?,
                    ),
                    (
                        "value",
                        CheckedValue::scalar(self.program, NormalizedValue::text(text))?,
                    ),
                ])
            }
            "core.list.get" => {
                let [list, index] = arguments.as_slice() else {
                    return Err(type_error("list lookup received a foreign arity"));
                };
                let NormalizedValue::I64(index) = index.raw() else {
                    return Err(type_error("list index is not an integer"));
                };
                let index = usize::try_from(*index).map_err(|_| {
                    trap_error(
                        "normalized_list_index",
                        "list index is negative or excessive",
                    )
                })?;
                list.list_get(index)
            }
            "core.list.append" => {
                let [list, child] = take_intrinsic_arguments::<2>(
                    arguments,
                    "list append received a foreign arity",
                )?;
                let mut work = std::mem::take(&mut self.observation.value_work);
                let program = self.program;
                let maximum = self
                    .policy
                    .maximum_collection_items
                    .unwrap_or(super::super::list::MAXIMUM_LENGTH as u64);
                let result = list.append(program, child, &mut work, maximum, &mut |charge| {
                    self.charge_list(charge)
                });
                self.observation.value_work = work;
                result
            }
            "core.option.some" => {
                let [child] = take_intrinsic_arguments::<1>(
                    arguments,
                    "option construction received a foreign arity",
                )?;
                self.charge_collection(1, std::mem::size_of::<NormalizedValue>())?;
                CheckedValue::option(self.program, Some(child), &mut self.observation.value_work)
            }
            "core.option.none" => {
                if !arguments.is_empty() {
                    return Err(type_error("empty option received arguments"));
                }
                CheckedValue::option(self.program, None, &mut self.observation.value_work)
            }
            "core.option.get-or" => {
                let [option, fallback] = take_intrinsic_arguments::<2>(
                    arguments,
                    "option lookup received a foreign arity",
                )?;
                Ok(option.option_get()?.unwrap_or(fallback))
            }
            "core.map.get" | "core.map.get-or" | "core.map.contains" | "core.map.insert"
            | "core.map.remove" | "core.map.entries" => self.checked_map(implementation, arguments),
            "core.f64.parse-result" => {
                let [value] = arguments.as_slice() else {
                    return Err(type_error("F64 parser received a foreign arity"));
                };
                let NormalizedValue::Text(value) = value.raw() else {
                    return Err(type_error("F64 parser received a foreign value"));
                };
                let parsed = Binary64::parse(value);
                self.intrinsic_record([
                    (
                        "valid",
                        CheckedValue::scalar(
                            self.program,
                            NormalizedValue::Bool(parsed.is_some()),
                        )?,
                    ),
                    (
                        "value",
                        CheckedValue::scalar(
                            self.program,
                            NormalizedValue::F64(
                                parsed.unwrap_or_else(|| Binary64::from_float(0.0)),
                            ),
                        )?,
                    ),
                ])
            }
            "core.f64.to-i64-result" => {
                let [value] = arguments.as_slice() else {
                    return Err(type_error("F64 conversion received a foreign arity"));
                };
                let NormalizedValue::F64(value) = value.raw() else {
                    return Err(type_error("F64 conversion received a foreign value"));
                };
                let converted = f64_to_i64(value.to_float());
                self.intrinsic_record([
                    (
                        "valid",
                        CheckedValue::scalar(
                            self.program,
                            NormalizedValue::Bool(converted.is_some()),
                        )?,
                    ),
                    (
                        "value",
                        CheckedValue::scalar(
                            self.program,
                            NormalizedValue::I64(converted.unwrap_or_default()),
                        )?,
                    ),
                ])
            }
            "core.i64.parse-result" => {
                let [value] = arguments.as_slice() else {
                    return Err(type_error("integer parser received a foreign arity"));
                };
                let NormalizedValue::Text(value) = value.raw() else {
                    return Err(type_error("integer parser received a foreign value"));
                };
                let parsed = parse_normalized_i64(value);
                self.intrinsic_record([
                    (
                        "valid",
                        CheckedValue::scalar(
                            self.program,
                            NormalizedValue::Bool(parsed.is_some()),
                        )?,
                    ),
                    (
                        "value",
                        CheckedValue::scalar(
                            self.program,
                            NormalizedValue::I64(parsed.unwrap_or_default()),
                        )?,
                    ),
                ])
            }
            "core.json.decode-or" | "core.data.decode-or" => {
                let [bytes, fallback] =
                    take_intrinsic_arguments::<2>(arguments, "decoder received a foreign arity")?;
                let NormalizedValue::Bytes(bytes) = bytes.raw() else {
                    return Err(type_error("decoder input is not bytes"));
                };
                let substitutions = function
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(types.iter().copied())
                    .collect();
                let ty = function
                    .parameters
                    .get(1)
                    .and_then(|parameter| {
                        self.program
                            .substitute_type(parameter.ty, &substitutions, 0)
                    })
                    .ok_or_else(|| type_error("decoder fallback has no exact type"))?;
                let decoded = if implementation == "core.json.decode-or" {
                    super::super::codec::decode_typed_with_control(
                        self.program,
                        bytes,
                        ty,
                        JsonLimits::default(),
                        self.control,
                    )
                } else {
                    super::super::data_codec::decode_typed_with_control(
                        self.program,
                        bytes,
                        ty,
                        self.control,
                    )
                };
                let (valid, value, error) = match decoded {
                    Ok(raw) => (true, self.admit(raw, ty, None, false)?, String::new()),
                    Err(error)
                        if matches!(
                            error.class,
                            DiagnosticClass::Cancelled | DiagnosticClass::Resource
                        ) =>
                    {
                        return Err(normalized_json_error(error));
                    }
                    Err(error) => (false, fallback, error.code),
                };
                if implementation == "core.data.decode-or" {
                    return Ok(value);
                }
                self.charge_allocation(error.len() as u64)?;
                self.intrinsic_record([
                    (
                        "error",
                        CheckedValue::scalar(self.program, NormalizedValue::text(error))?,
                    ),
                    (
                        "valid",
                        CheckedValue::scalar(self.program, NormalizedValue::Bool(valid))?,
                    ),
                    ("value", value),
                ])
            }
            _ => {
                // The remaining closed operations create scalars. In particular length reads
                // an immediate collection header; scalar construction cannot certify a subtree.
                // Ordinary intrinsic arities fit this inline buffer. Move their already
                // admitted payloads without allocating a second argument vector; retain the
                // same raw implementation and scalar-result admission for both paths.
                let inline = arguments.len() <= 4;
                #[cfg(test)]
                let inline = inline && !self_tail_locals_tests::force_fresh_call_arguments();
                let raw = if inline {
                    let count = arguments.len();
                    let mut raw = std::array::from_fn::<_, 4, _>(|_| NormalizedValue::Unit);
                    for (slot, argument) in raw.iter_mut().zip(arguments.drain(..)) {
                        *slot = argument.into_raw();
                    }
                    call_core_intrinsic_borrowed(
                        self.program,
                        function,
                        implementation,
                        types,
                        &raw[..count],
                        self.control,
                    )?
                } else {
                    call_core_intrinsic(
                        self.program,
                        function,
                        implementation,
                        types,
                        arguments.drain(..).map(CheckedValue::into_raw).collect(),
                        self.control,
                    )?
                };
                let value = CheckedValue::scalar(self.program, raw)?;
                self.charge_external_value(value.raw())?;
                Ok(value)
            }
        }
    }

    pub(super) fn checked_map(
        &mut self,
        implementation: &str,
        arguments: &mut Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let mut arguments = arguments.drain(..);
        let map = arguments
            .next()
            .ok_or_else(|| type_error("map intrinsic has no map"))?;
        let NormalizedValue::Map(entries) = map.raw() else {
            return Err(type_error("map intrinsic received a foreign map"));
        };
        if implementation == "core.map.entries" {
            if arguments.next().is_some() {
                return Err(type_error("map entries has foreign arity"));
            }
            self.charge_collection(entries.len(), std::mem::size_of::<CheckedValue>())?;
            let mut children = Vec::with_capacity(entries.len());
            let mut cursor = map.map_entries(self.program)?;
            while let Some((key, value)) = cursor.next(&mut |charge| self.charge_map(charge))? {
                let mut work = std::mem::take(&mut self.observation.value_work);
                let program = self.program;
                let entry =
                    CheckedValue::map_entry(program, key, value, &mut work, &mut |charge| {
                        self.charge_map(charge)
                    });
                self.observation.value_work = work;
                children.push(entry?);
            }
            return self.construct_list(children);
        }
        let key = arguments
            .next()
            .ok_or_else(|| type_error("map intrinsic omits its key"))?;
        let key = self.checked_map_key(key)?;
        let third = arguments.next();
        if arguments.next().is_some() {
            return Err(type_error("map intrinsic has foreign arity"));
        }
        match (implementation, third) {
            ("core.map.get", None) => map
                .map_get(&key, &mut |charge| self.charge_map(charge))?
                .ok_or_else(|| trap_error("normalized_map_key_absent", "map lookup key is absent")),
            ("core.map.get-or", Some(fallback)) => Ok(map
                .map_get(&key, &mut |charge| self.charge_map(charge))?
                .unwrap_or(fallback)),
            ("core.map.contains", None) => CheckedValue::scalar(
                self.program,
                NormalizedValue::Bool(entries.contains_key(&key)),
            ),
            ("core.map.insert", Some(value)) => self.edit_checked_map(map, key, Some(value)),
            ("core.map.remove", None) => self.edit_checked_map(map, key, None),
            _ => Err(type_error("map intrinsic has foreign arity")),
        }
    }
}

// Check exact arity before moving the first slot. The intrinsic arities are at most
// three, so shifting these short buffers retains capacity without another allocation.
fn take_intrinsic_arguments<const N: usize>(
    arguments: &mut Vec<CheckedValue>,
    message: &'static str,
) -> Result<[CheckedValue; N], ExecutionError> {
    if arguments.len() != N {
        return Err(type_error(message));
    }
    Ok(std::array::from_fn(|_| arguments.remove(0)))
}
