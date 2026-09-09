//! Closed intrinsic propagation; arbitrary hosts enter through raw result admission.

use super::*;

impl Machine<'_> {
    fn charge_list(&mut self, charge: super::super::list::Charge) -> Result<(), ExecutionError> {
        self.control.check()?;
        let next = self
            .observation
            .collection_items
            .checked_add(charge.slots)
            .filter(|next| *next <= self.policy.maximum_collection_items)
            .ok_or_else(|| {
                resource_error(
                    "normalized_collection_items",
                    "persistent list storage exceeds collection items",
                )
            })?;
        self.observation.collection_items = next;
        self.charge_allocation(charge.bytes)
    }

    pub(super) fn construct_list(
        &mut self,
        items: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let mut work = std::mem::take(&mut self.observation.value_work);
        let program = self.program;
        let maximum = self.policy.maximum_collection_items;
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
            substitutions,
            program: self.program,
            resources: self.resources,
            control: self.control,
            policy: self.policy,
            work: &mut self.observation.value_work,
            allocated: &mut self.observation.allocated_bytes,
            allocation_charges: &mut self.observation.allocation_charges,
            items: &mut self.observation.collection_items,
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
            if self
                .program
                .substitute_type(*ty, &BTreeMap::new(), 0)
                .is_none()
            {
                return Err(type_error(
                    "raw invocation requires fully resolved exact types",
                ));
            }
            if *constraint == crate::platform::kernel::TypeParameterConstraints::CaptureSafe {
                checked::Admission {
                    substitutions: &BTreeMap::new(),
                    program: self.program,
                    resources: self.resources,
                    control: self.control,
                    policy: self.policy,
                    work: &mut self.observation.value_work,
                    allocated: &mut self.observation.allocated_bytes,
                    allocation_charges: &mut self.observation.allocation_charges,
                    items: &mut self.observation.collection_items,
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
            Some(TypeForm::Function { parameters, .. }) => parameters.clone(),
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
        arguments: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        self.control.check()?;
        match implementation {
            "identity_host" => match arguments.len() {
                0 => CheckedValue::scalar(self.program, NormalizedValue::Unit),
                1 => arguments
                    .into_iter()
                    .next()
                    .ok_or_else(|| type_error("identity arity changed")),
                _ => Err(type_error("identity host received a foreign arity")),
            },
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
                let [list, child]: [CheckedValue; 2] = arguments
                    .try_into()
                    .map_err(|_| type_error("list append received a foreign arity"))?;
                let mut work = std::mem::take(&mut self.observation.value_work);
                let program = self.program;
                let maximum = self.policy.maximum_collection_items;
                let result = list.append(program, child, &mut work, maximum, &mut |charge| {
                    self.charge_list(charge)
                });
                self.observation.value_work = work;
                result
            }
            "core.option.some" => {
                let [child]: [CheckedValue; 1] = arguments
                    .try_into()
                    .map_err(|_| type_error("option construction received a foreign arity"))?;
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
                let [option, fallback]: [CheckedValue; 2] = arguments
                    .try_into()
                    .map_err(|_| type_error("option lookup received a foreign arity"))?;
                Ok(option.option_get()?.unwrap_or(fallback))
            }
            "core.map.get" | "core.map.get-or" | "core.map.contains" | "core.map.insert"
            | "core.map.remove" | "core.map.entries" => self.checked_map(implementation, arguments),
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
                let [bytes, fallback]: [CheckedValue; 2] = arguments
                    .try_into()
                    .map_err(|_| type_error("decoder received a foreign arity"))?;
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
                    decode_typed(self.program, bytes, ty, JsonLimits::default())
                } else {
                    super::super::data_codec::decode_typed(self.program, bytes, ty)
                };
                let (valid, value, error) = match decoded {
                    Ok(raw) => (true, self.admit(raw, ty, None, false)?, String::new()),
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
                let raw = call_core_intrinsic(
                    self.program,
                    function,
                    implementation,
                    types,
                    arguments.into_iter().map(CheckedValue::into_raw).collect(),
                )?;
                let value = CheckedValue::scalar(self.program, raw)?;
                self.charge_external_value(value.raw())?;
                Ok(value)
            }
        }
    }

    pub(super) fn checked_map(
        &mut self,
        implementation: &str,
        arguments: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let mut arguments = arguments.into_iter();
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
            self.charge_collection(entries.len(), std::mem::size_of::<NormalizedValue>())?;
            let mut children = Vec::with_capacity(entries.len());
            for key in entries.keys() {
                self.control.check()?;
                let value = map
                    .map_get(key)?
                    .ok_or_else(|| type_error("map key disappeared"))?;
                children.push(self.intrinsic_record([
                    ("key", CheckedValue::scalar(self.program, key.to_value())?),
                    ("value", value),
                ])?);
            }
            return self.construct_list(children);
        }
        let key = arguments
            .next()
            .and_then(|value| NormalizedMapKey::from_value(value.into_raw()))
            .ok_or_else(|| {
                trap_error("normalized_map_key", "map key is not an ordered primitive")
            })?;
        let third = arguments.next();
        if arguments.next().is_some() {
            return Err(type_error("map intrinsic has foreign arity"));
        }
        match (implementation, third) {
            ("core.map.get", None) => map
                .map_get(&key)?
                .ok_or_else(|| trap_error("normalized_map_key_absent", "map lookup key is absent")),
            ("core.map.get-or", Some(fallback)) => Ok(map.map_get(&key)?.unwrap_or(fallback)),
            ("core.map.contains", None) => CheckedValue::scalar(
                self.program,
                NormalizedValue::Bool(entries.contains_key(&key)),
            ),
            ("core.map.insert", Some(value)) => {
                self.charge_collection(
                    entries
                        .len()
                        .saturating_add(usize::from(!entries.contains_key(&key))),
                    std::mem::size_of::<(NormalizedMapKey, NormalizedValue)>(),
                )?;
                map.edit_map(
                    self.program,
                    key,
                    Some(value),
                    &mut self.observation.value_work,
                )
            }
            ("core.map.remove", None) => {
                self.charge_collection(
                    entries
                        .len()
                        .saturating_sub(usize::from(entries.contains_key(&key))),
                    std::mem::size_of::<(NormalizedMapKey, NormalizedValue)>(),
                )?;
                map.edit_map(self.program, key, None, &mut self.observation.value_work)
            }
            _ => Err(type_error("map intrinsic has foreign arity")),
        }
    }
}
