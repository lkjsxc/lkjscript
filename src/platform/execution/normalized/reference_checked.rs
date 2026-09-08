//! Reference-owned value proofs. Only canonical schemas and owner reads establish eligibility.

use super::super::value::ValueOrigin;
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Ownership {
    Ordinary,
    Capability,
    AffineVariant,
}

#[derive(Debug)]
pub(super) struct Value {
    datum: NormalizedValue,
    preparation: ValueOrigin,
    ownership: Ownership,
}

impl Value {
    pub(super) fn raw(&self) -> &NormalizedValue {
        &self.datum
    }
    pub(super) fn release(self) -> NormalizedValue {
        self.datum
    }

    pub(super) fn ownership(
        &self,
        schema: &BoundReferenceSchema,
        work: &mut super::super::value::ValueWork,
    ) -> Result<Ownership, ExecutionError> {
        #[cfg(test)]
        super::super::value_oracle::forced_descendant_work(&self.datum, work);
        work.classification_decisions = work.classification_decisions.saturating_add(1);
        if self.preparation != schema.value_origin {
            return Err(reject(
                "checked reference slot belongs to another preparation; admit it at the selected invocation",
            ));
        }
        Ok(self.ownership)
    }

    pub(super) fn duplicate(&self, use_mode: ParameterUse) -> Result<Self, ExecutionError> {
        match (self.ownership, use_mode) {
            (Ownership::Ordinary, ParameterUse::Unrestricted)
            | (Ownership::Capability, ParameterUse::Borrow) => Ok(Self {
                datum: self.datum.clone(),
                preparation: self.preparation,
                ownership: self.ownership,
            }),
            _ => Err(reference_error(
                "normalized_reference_local_resource_use",
                "affine reference owner cannot be ordinarily duplicated; use its exact borrow or consume",
            )),
        }
    }

    pub(super) fn primitive(
        schema: &BoundReferenceSchema,
        datum: NormalizedValue,
    ) -> Result<Self, ExecutionError> {
        match datum {
            NormalizedValue::Unit
            | NormalizedValue::I64(_)
            | NormalizedValue::Bool(_)
            | NormalizedValue::Text(_)
            | NormalizedValue::StaticText(_)
            | NormalizedValue::Bytes(_) => Ok(Self {
                datum,
                preparation: schema.value_origin,
                ownership: Ownership::Ordinary,
            }),
            _ => Err(reject(
                "primitive reference constructor received structured data or authority; use its checked constructor",
            )),
        }
    }

    pub(super) fn callable(
        schema: &BoundReferenceSchema,
        function: FunctionIndex,
        type_arguments: Arc<[TypeObjectDigest]>,
    ) -> Result<Self, ExecutionError> {
        if function.1 != schema.value_origin || schema.functions.get(function.0 as usize).is_none()
        {
            return Err(reject(
                "callable identity is foreign; select its canonical declaration",
            ));
        }
        Ok(Self {
            datum: NormalizedValue::Function {
                function,
                type_arguments,
            },
            preparation: schema.value_origin,
            ownership: Ownership::Ordinary,
        })
    }

    pub(super) fn project_field(
        &self,
        selector: FieldSelector,
        schema: &BoundReferenceSchema,
    ) -> Result<Self, ExecutionError> {
        let selected = match (self.raw(), selector) {
            (
                NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }),
                FieldSelector::Nominal(reference),
            ) => {
                let offset = schema
                    .records
                    .get(layout.0 as usize)
                    .and_then(|definition| {
                        definition
                            .fields
                            .iter()
                            .position(|field| field.reference == reference)
                    });
                offset.and_then(|offset| fields.get(offset))
            }
            (
                NormalizedValue::Record(NormalizedRecord::Structural { fields }),
                FieldSelector::Structural(name),
            ) => fields
                .binary_search_by(|field| field.0.cmp(&name))
                .ok()
                .and_then(|offset| fields.get(offset))
                .map(|field| &field.1),
            _ => None,
        }
        .ok_or_else(|| reference_type_error("field projection has a foreign record or field"))?;
        Ok(Self {
            datum: selected.clone(),
            preparation: self.preparation,
            ownership: Ownership::Ordinary,
        })
    }

    pub(super) fn at(&self, index: usize) -> Result<Self, ExecutionError> {
        let NormalizedValue::List(items) = self.raw() else {
            return Err(reference_type_error("list projection has a foreign value"));
        };
        let datum = items
            .get(index)
            .ok_or_else(|| reference_trap("reference_list_index", "list index is out of bounds"))?
            .clone();
        Ok(Self {
            datum,
            preparation: self.preparation,
            ownership: Ownership::Ordinary,
        })
    }

    pub(super) fn optional(&self) -> Result<Option<Self>, ExecutionError> {
        let NormalizedValue::Option(item) = self.raw() else {
            return Err(reference_type_error(
                "option projection has a foreign value",
            ));
        };
        Ok(item.as_deref().map(|item| Self {
            datum: item.clone(),
            preparation: self.preparation,
            ownership: Ownership::Ordinary,
        }))
    }

    pub(super) fn lookup(&self, key: &NormalizedMapKey) -> Result<Option<Self>, ExecutionError> {
        let NormalizedValue::Map(entries) = self.raw() else {
            return Err(reference_type_error("map projection has a foreign value"));
        };
        Ok(entries.get(key).map(|item| Self {
            datum: item.clone(),
            preparation: self.preparation,
            ownership: Ownership::Ordinary,
        }))
    }

    pub(super) fn open_variant(
        self,
        schema: &BoundReferenceSchema,
    ) -> Result<(VariantLayoutIndex, u32, Option<Self>), ExecutionError> {
        let NormalizedValue::Variant {
            layout,
            case,
            payload,
        } = self.datum
        else {
            return Err(reference_type_error("match value is not a variant"));
        };
        let selected = schema.variants.get(layout.0 as usize).and_then(|variant| variant.cases.get(case as usize)).ok_or_else(|| reject("matched variant escaped its canonical case inventory; prepare accepted meaning"))?;
        let ownership = if selected
            .payload
            .and_then(|ty| schema.types.get(&ty))
            .is_some_and(|ty| matches!(ty.form, TypeForm::CapabilityResource { .. }))
        {
            Ownership::Capability
        } else {
            Ownership::Ordinary
        };
        Ok((
            layout,
            case,
            payload.map(|datum| Self {
                datum: *datum,
                preparation: self.preparation,
                ownership,
            }),
        ))
    }
}

impl ReferenceState<'_> {
    pub(super) fn ordinary_children<'v>(
        &mut self,
        children: impl IntoIterator<Item = &'v Value>,
    ) -> Result<(), ExecutionError> {
        for child in children {
            self.observation.value_work.constructor_child_visits = self
                .observation
                .value_work
                .constructor_child_visits
                .saturating_add(1);
            if child.ownership(&self.schema, &mut self.observation.value_work)?
                != Ownership::Ordinary
            {
                return Err(reject(
                    "reference aggregate would contain affine authority; transfer the owner only at its exact boundary",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn list_value(&mut self, items: Vec<Value>) -> Result<Value, ExecutionError> {
        self.ordinary_children(&items)?;
        Ok(Value {
            datum: NormalizedValue::List(Arc::new(items.into_iter().map(Value::release).collect())),
            preparation: self.schema.value_origin,
            ownership: Ownership::Ordinary,
        })
    }

    pub(super) fn record_value(
        &mut self,
        layout: Option<RecordLayoutIndex>,
        fields: Vec<(Name, Value)>,
    ) -> Result<Value, ExecutionError> {
        self.ordinary_children(fields.iter().map(|(_, child)| child))?;
        let datum = match layout {
            Some(layout) => {
                let definition = self.schema.records.get(layout.0 as usize).filter(|_| layout.1 == self.schema.value_origin).ok_or_else(|| reject("record constructor has a foreign preparation; select its canonical layout"))?;
                if definition.fields.len() != fields.len()
                    || definition
                        .fields
                        .iter()
                        .zip(&fields)
                        .any(|(field, (name, _))| field.name != *name)
                {
                    return Err(reject(
                        "record constructor disagrees with canonical fields; supply the exact ordered shape",
                    ));
                }
                NormalizedRecord::Nominal {
                    layout,
                    fields: Arc::new(
                        fields
                            .into_iter()
                            .map(|(_, value)| value.release())
                            .collect(),
                    ),
                }
            }
            None => {
                if fields.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
                    return Err(reject(
                        "structural fields are not unique and ordered; supply canonical field names",
                    ));
                }
                NormalizedRecord::Structural {
                    fields: Arc::new(
                        fields
                            .into_iter()
                            .map(|(name, value)| (name, value.release()))
                            .collect(),
                    ),
                }
            }
        };
        Ok(Value {
            datum: NormalizedValue::Record(datum),
            preparation: self.schema.value_origin,
            ownership: Ownership::Ordinary,
        })
    }

    pub(super) fn variant_value(
        &mut self,
        layout: VariantLayoutIndex,
        case: u32,
        payload: Option<Value>,
    ) -> Result<Value, ExecutionError> {
        let selected = self
            .schema
            .variants
            .get(layout.0 as usize)
            .filter(|_| layout.1 == self.schema.value_origin)
            .and_then(|variant| variant.cases.get(case as usize))
            .ok_or_else(|| {
                reject("variant constructor has a foreign case; select the exact canonical case")
            })?;
        if selected.payload.is_some() != payload.is_some() {
            return Err(reject(
                "variant payload presence disagrees with its case; supply the declared payload",
            ));
        }
        if let Some(child) = &payload {
            self.observation.value_work.constructor_child_visits = self
                .observation
                .value_work
                .constructor_child_visits
                .saturating_add(1);
            let expected = if selected
                .payload
                .and_then(|ty| self.schema.types.get(&ty))
                .is_some_and(|ty| matches!(ty.form, TypeForm::CapabilityResource { .. }))
            {
                Ownership::Capability
            } else {
                Ownership::Ordinary
            };
            if child.ownership(&self.schema, &mut self.observation.value_work)? != expected {
                return Err(reject(
                    "variant child has foreign ownership; supply its exact direct payload",
                ));
            }
        }
        let ownership = if self.schema.affine_variants[layout.0 as usize] {
            Ownership::AffineVariant
        } else {
            Ownership::Ordinary
        };
        Ok(Value {
            datum: NormalizedValue::Variant {
                layout,
                case,
                payload: payload.map(|value| Box::new(value.release())),
            },
            preparation: self.schema.value_origin,
            ownership,
        })
    }

    pub(super) fn option_value(&mut self, child: Option<Value>) -> Result<Value, ExecutionError> {
        self.ordinary_children(child.iter())?;
        Ok(Value {
            datum: NormalizedValue::Option(child.map(|value| Box::new(value.release()))),
            preparation: self.schema.value_origin,
            ownership: Ownership::Ordinary,
        })
    }

    pub(super) fn map_value(
        &mut self,
        entries: BTreeMap<NormalizedMapKey, Value>,
    ) -> Result<Value, ExecutionError> {
        self.ordinary_children(entries.values())?;
        Ok(Value {
            datum: NormalizedValue::Map(Arc::new(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, value.release()))
                    .collect(),
            )),
            preparation: self.schema.value_origin,
            ownership: Ownership::Ordinary,
        })
    }

    pub(super) fn append_value(
        &mut self,
        list: Value,
        child: Value,
    ) -> Result<Value, ExecutionError> {
        self.ordinary_children([&list, &child])?;
        let NormalizedValue::List(items) = list.datum else {
            return Err(reference_type_error("list append received a foreign value"));
        };
        let mut output = items.as_ref().clone();
        output.push(child.release());
        Ok(Value {
            datum: NormalizedValue::List(Arc::new(output)),
            preparation: list.preparation,
            ownership: Ownership::Ordinary,
        })
    }

    pub(super) fn update_map_value(
        &mut self,
        map: Value,
        key: NormalizedMapKey,
        child: Option<Value>,
    ) -> Result<Value, ExecutionError> {
        self.ordinary_children(std::iter::once(&map).chain(child.iter()))?;
        let NormalizedValue::Map(entries) = map.datum else {
            return Err(reference_type_error("map update received a foreign value"));
        };
        let mut output = entries.as_ref().clone();
        match child {
            Some(child) => {
                output.insert(key, child.release());
            }
            None => {
                output.remove(&key);
            }
        }
        Ok(Value {
            datum: NormalizedValue::Map(Arc::new(output)),
            preparation: map.preparation,
            ownership: Ownership::Ordinary,
        })
    }
}

impl ReferenceState<'_> {
    pub(super) fn admit_raw(
        &mut self,
        datum: NormalizedValue,
        expected: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        authority: Option<RequirementReference>,
        input: bool,
    ) -> Result<Value, ExecutionError> {
        match self.inspect_raw(&datum, expected, bindings, authority, input) {
            Ok(ownership) => Ok(Value {
                datum,
                preparation: self.schema.value_origin,
                ownership,
            }),
            Err(mut error) => {
                super::super::value::release_raw_values(vec![datum]);
                error.message = format!(
                    "{} reference value admission: {}",
                    if input {
                        "invocation input"
                    } else {
                        "raw result"
                    },
                    error.message
                );
                Err(error)
            }
        }
    }

    fn inspect_raw(
        &mut self,
        datum: &NormalizedValue,
        expected: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        authority: Option<RequirementReference>,
        input: bool,
    ) -> Result<Ownership, ExecutionError> {
        self.control.check()?;
        let mut ownership = None;
        // One independent depth-first worklist. No reference admission calls a VM certifier.
        let mut visits = vec![(datum, expected, 0_u16, true)];
        self.charge_allocation(
            std::mem::size_of::<(&NormalizedValue, TypeObjectDigest, u16, bool)>() as u64,
        )?;
        while let Some((node, mut expected, depth, owner_position)) = visits.pop() {
            self.control.check()?;
            let counter = if input {
                &mut self.observation.value_work.input_admission_nodes
            } else {
                &mut self.observation.value_work.raw_result_admission_nodes
            };
            *counter = counter.saturating_add(1);
            if depth > 256 {
                return Err(reference_resource(
                    "normalized_reference_value_depth",
                    "reference input exceeds depth 256; reduce its nesting",
                ));
            }
            let node_ownership = self.raw_ownership(node)?;
            if depth == 0 {
                ownership = Some(node_ownership);
            }
            if !owner_position && node_ownership != Ownership::Ordinary {
                return Err(reject(
                    "raw container conceals affine ownership; remove the nested owner before invocation",
                ));
            }
            if let Some(TypeForm::TypeParameter { parameter }) =
                self.schema.types.get(&expected).map(|ty| &ty.form)
            {
                expected = *bindings.get(parameter).ok_or_else(|| reject("raw boundary has an unbound type parameter; supply all exact type arguments"))?;
            }
            let schema = Arc::clone(&self.schema);
            let ty = &schema
                .types
                .get(&expected)
                .ok_or_else(|| {
                    reject("raw boundary names an unknown type; use accepted canonical meaning")
                })?
                .form;
            let next_depth = depth + 1;
            match ty {
                TypeForm::Unit if matches!(node, NormalizedValue::Unit) => {}
                TypeForm::Bool if matches!(node, NormalizedValue::Bool(_)) => {}
                TypeForm::I64 if matches!(node, NormalizedValue::I64(_)) => {}
                TypeForm::Text | TypeForm::StaticText | TypeForm::Bytes => {
                    let length = match (ty, node) {
                        (TypeForm::Text, NormalizedValue::Text(text))
                        | (TypeForm::StaticText, NormalizedValue::StaticText(text)) => text.len(),
                        (TypeForm::Bytes, NormalizedValue::Bytes(bytes)) => bytes.len(),
                        _ => {
                            return Err(reject(
                                "raw scalar disagrees with its canonical type; provide the exact declared value",
                            ));
                        }
                    };
                    self.charge_allocation(length as u64)?;
                }
                TypeForm::Named { declaration } => match node {
                    NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }) => {
                        let definition = schema.records.get(layout.0 as usize).filter(|record| layout.1 == schema.value_origin && record.declaration == *declaration && record.fields.len() == fields.len())
                            .ok_or_else(|| reject("raw record has a foreign nominal identity or shape; decode against the selected program"))?;
                        self.charge_admission_children(
                            fields.len(),
                            std::mem::size_of::<NormalizedValue>(),
                        )?;
                        for (child, field) in fields.iter().zip(definition.fields.iter()).rev() {
                            visits.push((child, field.ty, next_depth, false));
                        }
                    }
                    NormalizedValue::Variant {
                        layout,
                        case,
                        payload,
                    } => {
                        let definition = schema.variants.get(layout.0 as usize).filter(|variant| layout.1 == schema.value_origin && variant.declaration == *declaration)
                            .and_then(|variant| variant.cases.get(*case as usize)).ok_or_else(|| reject("raw sum has a foreign nominal identity or case; decode its exact canonical layout"))?;
                        match (definition.payload, payload.as_deref()) {
                            (None, None) => {}
                            (Some(ty), Some(payload)) => {
                                self.charge_admission_children(
                                    1,
                                    std::mem::size_of::<NormalizedValue>(),
                                )?;
                                let direct = schema.types.get(&ty).is_some_and(|ty| {
                                    matches!(ty.form, TypeForm::CapabilityResource { .. })
                                });
                                visits.push((
                                    payload,
                                    ty,
                                    next_depth,
                                    owner_position
                                        && node_ownership == Ownership::AffineVariant
                                        && direct,
                                ));
                            }
                            _ => {
                                return Err(reject(
                                    "raw sum payload presence is foreign; provide the selected case's exact payload",
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(reject(
                            "raw nominal type is represented by a foreign value; use its exact record or sum",
                        ));
                    }
                },
                TypeForm::StructuralRecord { fields: expected } => {
                    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = node
                    else {
                        return Err(reject(
                            "raw structural record has a foreign representation; supply canonical fields",
                        ));
                    };
                    if fields.len() != expected.len() {
                        return Err(reject(
                            "raw structural record has foreign arity; supply all and only the declared fields",
                        ));
                    }
                    self.charge_admission_children(
                        fields.len(),
                        std::mem::size_of::<(Name, NormalizedValue)>(),
                    )?;
                    for ((name, child), field) in fields.iter().zip(expected).rev() {
                        if *name != field.name {
                            return Err(reject(
                                "raw record fields are foreign or noncanonical; supply the exact ordered names",
                            ));
                        }
                        self.charge_allocation(name.as_str().len() as u64)?;
                        visits.push((child, field.ty, next_depth, false));
                    }
                }
                TypeForm::List { item } => {
                    let NormalizedValue::List(items) = node else {
                        return Err(reject(
                            "raw list type received another value; supply a list",
                        ));
                    };
                    self.charge_admission_children(
                        items.len(),
                        std::mem::size_of::<NormalizedValue>(),
                    )?;
                    for child in items.iter().rev() {
                        visits.push((child, *item, next_depth, false));
                    }
                }
                TypeForm::Option { item } => {
                    let NormalizedValue::Option(child) = node else {
                        return Err(reject(
                            "raw option type received another value; supply an option",
                        ));
                    };
                    if let Some(child) = child.as_deref() {
                        self.charge_admission_children(1, std::mem::size_of::<NormalizedValue>())?;
                        visits.push((child, *item, next_depth, false));
                    }
                }
                TypeForm::Map {
                    key: key_type,
                    value,
                } => {
                    let NormalizedValue::Map(entries) = node else {
                        return Err(reject("raw map type received another value; supply a map"));
                    };
                    self.charge_admission_children(
                        entries.len(),
                        std::mem::size_of::<(NormalizedMapKey, NormalizedValue)>(),
                    )?;
                    let key_type = match schema.types.get(key_type).map(|ty| &ty.form) {
                        Some(TypeForm::TypeParameter { parameter }) => bindings
                            .get(parameter)
                            .and_then(|ty| schema.types.get(ty))
                            .map(|ty| &ty.form),
                        form => form,
                    }
                    .ok_or_else(|| {
                        reject("raw map key type is not bound; supply its exact type argument")
                    })?;
                    for (key, child) in entries.iter().rev() {
                        self.control.check()?;
                        if !matches!(
                            (key_type, key),
                            (TypeForm::I64, NormalizedMapKey::I64(_))
                                | (TypeForm::Bool, NormalizedMapKey::Bool(_))
                                | (
                                    TypeForm::Text | TypeForm::StaticText,
                                    NormalizedMapKey::Text(_)
                                )
                                | (TypeForm::Bytes, NormalizedMapKey::Bytes(_))
                        ) {
                            return Err(reject(
                                "raw map key has a foreign type; supply exact primitive keys",
                            ));
                        }
                        self.charge_allocation(reference_map_key_bytes(key))?;
                        visits.push((child, *value, next_depth, false));
                    }
                }
                TypeForm::CapabilityResource { interface } => {
                    let NormalizedValue::Resource(handle) = node else {
                        return Err(reject(
                            "raw capability type is not a direct handle; use the exact acquired owner",
                        ));
                    };
                    if authority.is_none() || node_ownership != Ownership::Capability {
                        return Err(reject(
                            "raw boundary cannot acquire capability authority; use an authorized exact operation",
                        ));
                    }
                    self.resources
                        .validate_admission(*handle, authority, Some(*interface))?;
                }
                TypeForm::Stream { item }
                    if schema
                        .types
                        .get(item)
                        .is_some_and(|ty| matches!(ty.form, TypeForm::Bytes)) =>
                {
                    let NormalizedValue::Resource(handle) = node else {
                        return Err(reject(
                            "raw stream has a foreign representation; use an acquired live stream",
                        ));
                    };
                    if handle.is_affine_capability() {
                        return Err(reject(
                            "raw stream conceals affine capability ownership; supply the declared resource kind",
                        ));
                    }
                    self.resources.validate_admission(*handle, None, None)?;
                }
                TypeForm::Function { parameters, result } => {
                    let NormalizedValue::Function {
                        function,
                        type_arguments,
                    } = node
                    else {
                        return Err(reject(
                            "raw callback has a foreign representation; select an exact callable",
                        ));
                    };
                    let declaration = schema.functions.get(function.0 as usize).copied().filter(|_| function.1 == schema.value_origin).ok_or_else(|| reject("raw callback belongs to another preparation; select the current callable"))?;
                    let signature = self.function_signature(declaration)?;
                    if !signature.pure
                        || signature.parameters.len() != parameters.len()
                        || signature.type_parameters.len() != type_arguments.len()
                        || signature
                            .parameters
                            .iter()
                            .any(|parameter| parameter.resource_requirement.is_some())
                    {
                        return Err(reject(
                            "raw callback has a foreign arity or effect; supply its exact pure signature",
                        ));
                    }
                    if type_arguments
                        .iter()
                        .any(|ty| schema.substitute_type(*ty, &BTreeMap::new(), 0).is_none())
                    {
                        return Err(reject(
                            "raw callback contains a foreign or unbound type argument; supply exact canonical types",
                        ));
                    }
                    self.charge_allocation(
                        (type_arguments.len()
                            * std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>())
                            as u64,
                    )?;
                    let actual_bindings = signature
                        .type_parameters
                        .iter()
                        .copied()
                        .zip(type_arguments.iter().copied())
                        .collect();
                    for (actual, expected) in signature
                        .parameters
                        .iter()
                        .zip(parameters)
                        .map(|(parameter, ty)| (parameter.ty, *ty))
                        .chain(std::iter::once((signature.result, *result)))
                    {
                        let actual = schema.instantiated_identity(actual, &actual_bindings, 0).ok_or_else(|| reject("raw callback has an unbound exact type; instantiate its declared parameters"))?;
                        let expected = schema.instantiated_identity(expected, bindings, 0).ok_or_else(|| reject("raw callback boundary has an unbound expected type; select an exact invocation"))?;
                        if actual != expected {
                            return Err(reject(
                                "raw callback has foreign parameter or result types; select the exact callable",
                            ));
                        }
                    }
                }
                _ => {
                    return Err(reject(
                        "raw value fails its exact type or authority boundary; provide a correctly typed value",
                    ));
                }
            }
        }
        let ownership = ownership.ok_or_else(|| {
            reject("reference admission did not visit its root; retry with an intact evaluator")
        })?;
        if ownership != Ownership::Ordinary && authority.is_none() {
            return Err(reject(
                "raw boundary cannot acquire affine ownership; use an authorized exact capability result",
            ));
        }
        self.charge_allocation(
            (std::mem::size_of::<Value>() - std::mem::size_of::<NormalizedValue>()) as u64,
        )?;
        Ok(ownership)
    }

    fn raw_ownership(&self, value: &NormalizedValue) -> Result<Ownership, ExecutionError> {
        Ok(match value {
            NormalizedValue::Resource(handle) if handle.is_affine_capability() => {
                Ownership::Capability
            }
            NormalizedValue::Variant { layout, .. } => {
                if layout.1 != self.schema.value_origin {
                    return Err(reject(
                        "raw variant belongs to a foreign preparation; decode it against this invocation",
                    ));
                }
                match self.schema.affine_variants.get(layout.0 as usize) {
                    Some(true) => Ownership::AffineVariant,
                    Some(false) => Ownership::Ordinary,
                    None => {
                        return Err(reject(
                            "raw variant has an unknown layout; supply an accepted nominal identity",
                        ));
                    }
                }
            }
            _ => Ownership::Ordinary,
        })
    }

    fn charge_admission_children(
        &mut self,
        count: usize,
        payload_bytes: usize,
    ) -> Result<(), ExecutionError> {
        self.charge_items(
            count,
            payload_bytes + std::mem::size_of::<(&NormalizedValue, TypeObjectDigest, u16, bool)>(),
        )
    }

    pub(super) fn admit_call_arguments(
        &mut self,
        declaration: DeclarationReference,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
    ) -> Result<Vec<Value>, ExecutionError> {
        let arguments = super::super::value::RawArguments::new(arguments);
        let signature = self.function_signature(declaration)?;
        if signature.parameters.len() != arguments.len()
            || signature.type_parameters.len() != types.len()
        {
            return Err(reference_type_error(
                "raw invocation arity disagrees with its canonical callable",
            ));
        }
        self.charge_allocation(
            (types.len() * std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()) as u64,
        )?;
        let bindings = signature
            .type_parameters
            .iter()
            .copied()
            .zip(types.iter().copied())
            .collect();
        arguments
            .into_iter()
            .zip(signature.parameters)
            .map(|(value, parameter)| {
                self.admit_raw(
                    value,
                    parameter.ty,
                    &bindings,
                    parameter.resource_requirement,
                    true,
                )
            })
            .collect()
    }

    pub(super) fn admit_port_arguments(
        &mut self,
        ty: TypeObjectDigest,
        arguments: Vec<NormalizedValue>,
    ) -> Result<Vec<Value>, ExecutionError> {
        let arguments = super::super::value::RawArguments::new(arguments);
        let schema = Arc::clone(&self.schema);
        let Some(TypeForm::Function { parameters, .. }) = schema.types.get(&ty).map(|ty| &ty.form)
        else {
            return Err(reference_type_error(
                "raw port boundary has no exact function type",
            ));
        };
        if parameters.len() != arguments.len() {
            return Err(reference_type_error(
                "raw port invocation has a foreign arity",
            ));
        }
        arguments
            .into_iter()
            .zip(parameters)
            .map(|(value, ty)| self.admit_raw(value, *ty, &BTreeMap::new(), None, true))
            .collect()
    }
}

fn reject(message: &'static str) -> ExecutionError {
    reference_error("normalized_reference_value_admission", message)
}
