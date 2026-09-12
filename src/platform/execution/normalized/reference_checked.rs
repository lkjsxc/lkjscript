//! Reference-owned value proofs. Only canonical schemas and owner reads establish eligibility.

use super::super::value::ValueOrigin;
use super::*;
use std::collections::BTreeSet;

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

type Invocation = (
    DeclarationReference,
    Arc<[TypeObjectDigest]>,
    Arc<[EffectRow]>,
    Vec<Value>,
);

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
        effect_arguments: Arc<[EffectRow]>,
        signature: &ReferenceSignature,
    ) -> Result<Self, ExecutionError> {
        if function.1 != schema.value_origin || schema.functions.get(function.0 as usize).is_none()
        {
            return Err(reject(
                "callable identity is foreign; select its canonical declaration",
            ));
        }
        if signature.type_parameters.len() != type_arguments.len()
            || signature.effect_parameters.len() != effect_arguments.len()
            || effect_arguments
                .iter()
                .any(|row| !row.is_closed() || row.validate().is_err())
            || type_arguments
                .iter()
                .any(|ty| schema.substitute_type(*ty, &BTreeMap::new(), 0).is_none())
        {
            return Err(reject(
                "named reference callable requires all resolved canonical type arguments",
            ));
        }
        if signature
            .type_parameter_constraints
            .iter()
            .zip(type_arguments.iter())
            .any(|(constraint, ty)| {
                *constraint == crate::platform::kernel::TypeParameterConstraints::CaptureSafe
                    && !schema.capture_safe_types.contains(ty)
            })
        {
            return Err(reject(
                "named canonical callable requires capture-safe type arguments",
            ));
        }
        Ok(Self {
            datum: NormalizedValue::Function {
                function,
                type_arguments,
                effect_arguments,
                bound_arguments: None,
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
        self.charge_allocation(
            items
                .len()
                .checked_mul(std::mem::size_of::<NormalizedValue>())
                .ok_or_else(|| {
                    reference_resource(
                        "normalized_list_storage",
                        "list construction metadata overflowed",
                    )
                })? as u64,
        )?;
        let list = super::super::list::List::from_items(
            items.into_iter().map(Value::release).collect(),
            self.policy
                .maximum_collection_items
                .unwrap_or(super::super::list::MAXIMUM_LENGTH as u64),
            &mut |charge| self.reserve_list(charge),
        )?;
        Ok(Value {
            datum: NormalizedValue::List(list),
            preparation: self.schema.value_origin,
            ownership: Ownership::Ordinary,
        })
    }

    fn reserve_list(&mut self, charge: super::super::list::Charge) -> Result<(), ExecutionError> {
        self.control.check()?;
        let slots = crate::platform::execution::cumulative_charge(
            self.observation.collection_items,
            charge.slots,
            self.policy.maximum_collection_items,
            "normalized_reference_collection_items",
            "list storage exceeds collection items",
        )?;
        self.observation.collection_items = slots;
        self.charge_allocation(charge.bytes)
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
        let output = items.append(
            child.release(),
            self.policy
                .maximum_collection_items
                .unwrap_or(super::super::list::MAXIMUM_LENGTH as u64),
            &mut |charge| self.reserve_list(charge),
        )?;
        Ok(Value {
            datum: NormalizedValue::List(output),
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

type ReferenceBindings = BTreeMap<TypeParameterId, TypeObjectDigest>;
type ReferenceVisit<'a> = (
    &'a NormalizedValue,
    TypeObjectDigest,
    u16,
    bool,
    Arc<ReferenceBindings>,
    bool,
);

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
        self.charge_reference_bindings(bindings.len())?;
        self.charge_allocation(std::mem::size_of::<ReferenceVisit<'_>>() as u64)?;
        let bindings = Arc::new(bindings.clone());
        self.inspect_reference_values(
            vec![(datum, expected, 0, true, bindings, false)],
            authority,
            input,
        )
    }

    fn inspect_reference_values(
        &mut self,
        mut visits: Vec<ReferenceVisit<'_>>,
        authority: Option<RequirementReference>,
        input: bool,
    ) -> Result<Ownership, ExecutionError> {
        let mut ownership = None;
        let mut admission_bytes = 0_u64;
        let mut admission_items = 0_u64;
        let mut capture_types = BTreeSet::new();
        while let Some((node, mut expected, depth, owner_position, bindings, captured)) =
            visits.pop()
        {
            self.control.check()?;
            let counter = if captured {
                &mut self.observation.value_work.capture_admission_nodes
            } else if input {
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
            if ownership.is_none() {
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
            let identity = self
                .schema
                .instantiated_identity(expected, &bindings, 0)
                .ok_or_else(|| reject("canonical raw type cannot be instantiated"))?;
            let application_free = self.schema.application_free_types.contains(&expected)
                && bindings
                    .values()
                    .all(|argument| self.schema.application_free_types.contains(argument));
            if !application_free && !self.schema.ordinary_types.contains(&identity) {
                return Err(reject("nominal type contains a live argument or member"));
            }
            if captured {
                self.check_capture_type(expected, &bindings, &mut capture_types)?;
                if node_ownership != Ownership::Ordinary
                    || matches!(node, NormalizedValue::Resource(_))
                {
                    return Err(reject(
                        "reference environment contains live or affine authority",
                    ));
                }
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
                    self.charge_admission_bytes(&mut admission_bytes, length as u64)?;
                }
                TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                    match node {
                        NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }) => {
                            let exact = schema
                                .instantiated_identity(expected, &bindings, 0)
                                .and_then(|ty| schema.record_instances.get(&ty).copied());
                            let definition = schema.records.get(layout.0 as usize).filter(|record| exact == Some(layout.0 as usize) && layout.1 == schema.value_origin && record.declaration == *declaration && record.fields.len() == fields.len())
                            .ok_or_else(|| reject("raw record has a foreign nominal identity or shape; decode against the selected program"))?;
                            self.charge_admission_children(
                                &mut admission_items,
                                &mut admission_bytes,
                                fields.len(),
                                std::mem::size_of::<NormalizedValue>(),
                            )?;
                            for (child, field) in fields.iter().zip(definition.fields.iter()).rev()
                            {
                                visits.push((
                                    child,
                                    field.ty,
                                    next_depth,
                                    false,
                                    Arc::clone(&bindings),
                                    captured,
                                ));
                            }
                        }
                        NormalizedValue::Variant {
                            layout,
                            case,
                            payload,
                        } => {
                            let exact = schema
                                .instantiated_identity(expected, &bindings, 0)
                                .and_then(|ty| schema.variant_instances.get(&ty).copied());
                            let definition = schema.variants.get(layout.0 as usize).filter(|variant| exact == Some(layout.0 as usize) && layout.1 == schema.value_origin && variant.declaration == *declaration)
                            .and_then(|variant| variant.cases.get(*case as usize)).ok_or_else(|| reject("raw sum has a foreign nominal identity or case; decode its exact canonical layout"))?;
                            match (definition.payload, payload.as_deref()) {
                                (None, None) => {}
                                (Some(ty), Some(payload)) => {
                                    self.charge_admission_children(
                                        &mut admission_items,
                                        &mut admission_bytes,
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
                                        Arc::clone(&bindings),
                                        captured,
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
                    }
                }
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
                        &mut admission_items,
                        &mut admission_bytes,
                        fields.len(),
                        std::mem::size_of::<(Name, NormalizedValue)>(),
                    )?;
                    for ((name, child), field) in fields.iter().zip(expected).rev() {
                        if *name != field.name {
                            return Err(reject(
                                "raw record fields are foreign or noncanonical; supply the exact ordered names",
                            ));
                        }
                        self.charge_admission_bytes(
                            &mut admission_bytes,
                            name.as_str().len() as u64,
                        )?;
                        visits.push((
                            child,
                            field.ty,
                            next_depth,
                            false,
                            Arc::clone(&bindings),
                            captured,
                        ));
                    }
                }
                TypeForm::List { item } => {
                    let NormalizedValue::List(items) = node else {
                        return Err(reject(
                            "raw list type received another value; supply a list",
                        ));
                    };
                    self.charge_admission_children(
                        &mut admission_items,
                        &mut admission_bytes,
                        items.len(),
                        std::mem::size_of::<NormalizedValue>(),
                    )?;
                    self.charge_admission_bytes(&mut admission_bytes, items.metadata_bytes()?)?;
                    for child in items.iter().rev() {
                        self.control.check()?;
                        visits.push((
                            child,
                            *item,
                            next_depth,
                            false,
                            Arc::clone(&bindings),
                            captured,
                        ));
                    }
                }
                TypeForm::Option { item } => {
                    let NormalizedValue::Option(child) = node else {
                        return Err(reject(
                            "raw option type received another value; supply an option",
                        ));
                    };
                    if let Some(child) = child.as_deref() {
                        self.charge_admission_children(
                            &mut admission_items,
                            &mut admission_bytes,
                            1,
                            std::mem::size_of::<NormalizedValue>(),
                        )?;
                        visits.push((
                            child,
                            *item,
                            next_depth,
                            false,
                            Arc::clone(&bindings),
                            captured,
                        ));
                    }
                }
                TypeForm::Result { ok, error } => {
                    let NormalizedValue::Result { success, value } = node else {
                        return Err(reject(
                            "raw result type received another value; supply its exact result case",
                        ));
                    };
                    self.charge_admission_children(
                        &mut admission_items,
                        &mut admission_bytes,
                        1,
                        std::mem::size_of::<NormalizedValue>(),
                    )?;
                    visits.push((
                        value,
                        if *success { *ok } else { *error },
                        next_depth,
                        false,
                        Arc::clone(&bindings),
                        captured,
                    ));
                }
                TypeForm::Map {
                    key: key_type,
                    value,
                } => {
                    let NormalizedValue::Map(entries) = node else {
                        return Err(reject("raw map type received another value; supply a map"));
                    };
                    self.charge_admission_children(
                        &mut admission_items,
                        &mut admission_bytes,
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
                        self.charge_admission_bytes(
                            &mut admission_bytes,
                            reference_map_key_bytes(key),
                        )?;
                        visits.push((
                            child,
                            *value,
                            next_depth,
                            false,
                            Arc::clone(&bindings),
                            captured,
                        ));
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
                TypeForm::Function { parameters, result }
                | TypeForm::TaskFunction {
                    parameters, result, ..
                } => {
                    let NormalizedValue::Function {
                        function,
                        type_arguments,
                        effect_arguments,
                        bound_arguments,
                    } = node
                    else {
                        return Err(reject(
                            "raw callback has a foreign representation; select an exact callable",
                        ));
                    };
                    let declaration = schema.functions.get(function.0 as usize).copied().filter(|_| function.1 == schema.value_origin).ok_or_else(|| reject("raw callback belongs to another preparation; select the current callable"))?;
                    let signature = self
                        .applied_signature(declaration, type_arguments, effect_arguments)
                        .map_err(|error| {
                            if error.code == "normalized_reference_type" {
                                reject(format!("raw callback signature: {}", error.message))
                            } else {
                                error
                            }
                        })?;
                    let kind_matches = match ty {
                        TypeForm::Function { .. } => signature.pure,
                        TypeForm::TaskFunction { effect, .. } => {
                            !signature.pure && signature.effect.row() == *effect
                        }
                        _ => false,
                    };
                    if !kind_matches
                        || bound_arguments
                            .as_ref()
                            .is_some_and(|prefix| prefix.is_empty())
                        || bound_arguments.as_ref().map_or(0, |prefix| prefix.len())
                            > crate::platform::kernel::contract::MAXIMUM_CHILDREN
                        || signature
                            .parameters
                            .len()
                            .checked_sub(bound_arguments.as_ref().map_or(0, |prefix| prefix.len()))
                            != Some(parameters.len())
                        || signature.type_parameters.len() != type_arguments.len()
                        || signature
                            .parameters
                            .iter()
                            .any(|parameter| parameter.resource_requirement.is_some())
                    {
                        return Err(reject(
                            "raw callback has a foreign arity or effect; supply its exact callable signature",
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
                    for (constraint, ty) in signature
                        .type_parameter_constraints
                        .iter()
                        .zip(type_arguments.iter())
                    {
                        if *constraint
                            == crate::platform::kernel::TypeParameterConstraints::CaptureSafe
                        {
                            self.check_capture_type(*ty, &BTreeMap::new(), &mut BTreeSet::new())?;
                        }
                    }
                    self.charge_reference_bindings(type_arguments.len())?;
                    let actual_bindings = Arc::new(
                        signature
                            .type_parameters
                            .iter()
                            .copied()
                            .zip(type_arguments.iter().copied())
                            .collect::<ReferenceBindings>(),
                    );
                    for (actual, expected) in signature
                        .parameters
                        .iter()
                        .skip(bound_arguments.as_ref().map_or(0, |prefix| prefix.len()))
                        .zip(parameters)
                        .map(|(parameter, ty)| (parameter.ty, *ty))
                        .chain(std::iter::once((signature.result, *result)))
                    {
                        let actual = schema.instantiated_identity(actual, &actual_bindings, 0).ok_or_else(|| reject("raw callback has an unbound exact type; instantiate its declared parameters"))?;
                        let expected = schema.instantiated_identity(expected, &bindings, 0).ok_or_else(|| reject("raw callback boundary has an unbound expected type; select an exact invocation"))?;
                        if actual != expected {
                            return Err(reject(
                                "raw callback has foreign parameter or result types; select the exact callable",
                            ));
                        }
                    }
                    if let Some(prefix) = bound_arguments {
                        self.charge_admission_children(
                            &mut admission_items,
                            &mut admission_bytes,
                            prefix.len(),
                            std::mem::size_of::<NormalizedValue>(),
                        )?;
                        self.charge_admission_bytes(
                            &mut admission_bytes,
                            (std::mem::size_of::<Vec<NormalizedValue>>()
                                + 2 * std::mem::size_of::<usize>())
                                as u64,
                        )?;
                        for (value, parameter) in
                            prefix.iter().zip(signature.parameters.iter()).rev()
                        {
                            visits.push((
                                value,
                                parameter.ty,
                                next_depth,
                                false,
                                Arc::clone(&actual_bindings),
                                true,
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
        self.charge_admission_bytes(
            &mut admission_bytes,
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
        admission_items: &mut u64,
        admission_bytes: &mut u64,
        count: usize,
        payload_bytes: usize,
    ) -> Result<(), ExecutionError> {
        *admission_items = crate::platform::execution::cumulative_charge(
            *admission_items,
            count as u64,
            Some(super::super::value::MAXIMUM_ADMISSION_ITEMS),
            "normalized_reference_collection_items",
            "single raw admission exceeds finite collection items",
        )?;
        let bytes = payload_bytes
            .checked_add(std::mem::size_of::<ReferenceVisit<'_>>())
            .and_then(|unit| count.checked_mul(unit))
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_allocation",
                    "raw admission storage size overflowed",
                )
            })?;
        *admission_bytes = crate::platform::execution::cumulative_charge(
            *admission_bytes,
            bytes as u64,
            Some(super::super::value::MAXIMUM_VALUE_ALLOCATION_BYTES),
            "normalized_reference_allocation",
            "single raw admission exceeds finite storage",
        )?;
        self.charge_items(
            count,
            payload_bytes + std::mem::size_of::<ReferenceVisit<'_>>(),
        )
    }

    fn charge_admission_bytes(
        &mut self,
        total: &mut u64,
        bytes: u64,
    ) -> Result<(), ExecutionError> {
        *total = crate::platform::execution::cumulative_charge(
            *total,
            bytes,
            Some(super::super::value::MAXIMUM_VALUE_ALLOCATION_BYTES),
            "normalized_reference_allocation",
            "single raw admission exceeds finite storage",
        )?;
        self.charge_allocation(bytes)
    }

    pub(super) fn admit_call_arguments(
        &mut self,
        declaration: DeclarationReference,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
    ) -> Result<Vec<Value>, ExecutionError> {
        let arguments = super::super::value::RawArguments::new(arguments);
        let signature = self.applied_signature(declaration, types, &[])?;
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
        for (constraint, ty) in signature
            .type_parameter_constraints
            .iter()
            .zip(types.iter())
        {
            self.control.check()?;
            if self
                .schema
                .substitute_type(*ty, &BTreeMap::new(), 0)
                .is_none()
            {
                return Err(reject(
                    "raw invocation requires resolved canonical type arguments",
                ));
            }
            if *constraint == crate::platform::kernel::TypeParameterConstraints::CaptureSafe {
                self.check_capture_type(*ty, &BTreeMap::new(), &mut BTreeSet::new())?;
            }
        }
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
        let Some(TypeForm::Function { parameters, .. } | TypeForm::TaskFunction { parameters, .. }) =
            schema.types.get(&ty).map(|ty| &ty.form)
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

impl ReferenceState<'_> {
    pub(super) fn bind_expression(
        &mut self,
        callee: Value,
        arguments: Vec<ExpressionId>,
        locals: &mut BTreeMap<LocalValueReference, Value>,
    ) -> Result<Value, ExecutionError> {
        self.control.check()?;
        if callee.ownership(&self.schema, &mut self.observation.value_work)? != Ownership::Ordinary
        {
            return Err(reject("binding requires an admitted immutable callable"));
        }
        let NormalizedValue::Function {
            function,
            type_arguments,
            effect_arguments,
            bound_arguments,
        } = callee.raw()
        else {
            return Err(reference_type_error("bind callee is not a function"));
        };
        let declaration = self.function_reference(*function)?;
        let signature = self.applied_signature(declaration, type_arguments, effect_arguments)?;
        if signature.type_parameters.len() != type_arguments.len()
            || signature
                .parameters
                .iter()
                .any(|parameter| parameter.resource_requirement.is_some())
        {
            return Err(reject(
                "binding requires the exact canonical callable signature",
            ));
        }
        let retained = bound_arguments.as_deref().map_or(&[][..], Vec::as_slice);
        let total = arguments
            .len()
            .checked_add(retained.len())
            .filter(|count| {
                *count <= signature.parameters.len()
                    && *count <= crate::platform::kernel::contract::MAXIMUM_CHILDREN
            })
            .ok_or_else(|| {
                reject("binding prefix exceeds the canonical target's remaining parameters")
            })?;
        if arguments.is_empty() {
            self.control.check()?;
            return Ok(callee);
        }
        self.charge_items(total, std::mem::size_of::<NormalizedValue>())?;
        self.charge_allocation(
            (std::mem::size_of::<Vec<NormalizedValue>>() + 2 * std::mem::size_of::<usize>()) as u64,
        )?;
        self.charge_reference_bindings(type_arguments.len())?;
        let bindings = Arc::new(
            signature
                .type_parameters
                .iter()
                .copied()
                .zip(type_arguments.iter().copied())
                .collect::<ReferenceBindings>(),
        );
        let mut prefix = Vec::with_capacity(total);
        prefix.extend(retained.iter().cloned());
        for expression in arguments {
            let value = self.evaluate(expression, locals)?;
            if value.ownership(&self.schema, &mut self.observation.value_work)?
                != Ownership::Ordinary
            {
                return Err(reject("capture contains affine ownership"));
            }
            let parameter = signature
                .parameters
                .get(prefix.len())
                .ok_or_else(|| reject("capture escaped its canonical prefix"))?;
            self.charge_allocation(std::mem::size_of::<ReferenceVisit<'_>>() as u64)?;
            self.inspect_reference_values(
                vec![(
                    value.raw(),
                    parameter.ty,
                    1,
                    false,
                    Arc::clone(&bindings),
                    true,
                )],
                None,
                false,
            )?;
            prefix.push(value.release());
        }
        self.control.check()?;
        Ok(Value {
            datum: NormalizedValue::Function {
                function: *function,
                type_arguments: Arc::clone(type_arguments),
                effect_arguments: Arc::clone(effect_arguments),
                bound_arguments: Some(Arc::new(prefix)),
            },
            preparation: self.schema.value_origin,
            ownership: Ownership::Ordinary,
        })
    }

    pub(super) fn callable_arguments(
        &mut self,
        callee: Value,
        arguments: Vec<Value>,
    ) -> Result<Invocation, ExecutionError> {
        self.control.check()?;
        let ownership = callee.ownership(&self.schema, &mut self.observation.value_work)?;
        if ownership != Ownership::Ordinary {
            return Err(reject("invocation requires a checked callable"));
        }
        let NormalizedValue::Function {
            function,
            type_arguments,
            effect_arguments,
            bound_arguments,
        } = callee.raw()
        else {
            return Err(reference_type_error("invoke callee is not a function"));
        };
        let declaration = self.function_reference(*function)?;
        let Some(prefix) = bound_arguments else {
            return Ok((
                declaration,
                Arc::clone(type_arguments),
                Arc::clone(effect_arguments),
                arguments,
            ));
        };
        let total = prefix
            .len()
            .checked_add(arguments.len())
            .filter(|total| *total <= crate::platform::kernel::contract::MAXIMUM_CHILDREN)
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_collection_items",
                    "complete callable arguments exceed the child bound",
                )
            })?;
        self.charge_allocation(
            total
                .checked_mul(std::mem::size_of::<Value>())
                .ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_allocation",
                        "callable arguments overflow allocation accounting",
                    )
                })? as u64,
        )?;
        let mut complete = Vec::with_capacity(total);
        for child in prefix.iter() {
            complete.push(Value {
                datum: child.clone(),
                preparation: callee.preparation,
                ownership: Ownership::Ordinary,
            });
        }
        complete.extend(arguments);
        Ok((
            declaration,
            Arc::clone(type_arguments),
            Arc::clone(effect_arguments),
            complete,
        ))
    }

    fn charge_reference_bindings(&mut self, count: usize) -> Result<(), ExecutionError> {
        let entry = std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
            + 3 * std::mem::size_of::<usize>();
        let bytes = count
            .checked_mul(entry)
            .and_then(|bytes| {
                bytes.checked_add(
                    std::mem::size_of::<ReferenceBindings>() + 2 * std::mem::size_of::<usize>(),
                )
            })
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_allocation",
                    "canonical binding metadata exceeds accounting range",
                )
            })?;
        self.charge_allocation(bytes as u64)
    }

    fn check_capture_type(
        &mut self,
        root: TypeObjectDigest,
        bindings: &ReferenceBindings,
        checked: &mut BTreeSet<TypeObjectDigest>,
    ) -> Result<(), ExecutionError> {
        self.charge_allocation(std::mem::size_of::<(TypeObjectDigest, u16)>() as u64)?;
        let schema = Arc::clone(&self.schema);
        let mut types = vec![(root, 0_u16)];
        while let Some((mut ty, depth)) = types.pop() {
            self.control.check()?;
            self.observation.value_work.capture_admission_nodes = self
                .observation
                .value_work
                .capture_admission_nodes
                .saturating_add(1);
            if depth > 256 {
                return Err(reference_resource(
                    "normalized_reference_value_depth",
                    "capture type exceeds depth 256",
                ));
            }
            if let Some(TypeForm::TypeParameter { parameter }) =
                schema.types.get(&ty).map(|object| &object.form)
            {
                ty = *bindings
                    .get(parameter)
                    .ok_or_else(|| reject("capture stores an unresolved type parameter"))?;
            }
            let identity = schema
                .instantiated_identity(ty, bindings, 0)
                .ok_or_else(|| reject("capture type cannot be resolved from canonical bindings"))?;
            if checked.contains(&identity) {
                continue;
            }
            self.charge_allocation(
                (std::mem::size_of::<TypeObjectDigest>() + 3 * std::mem::size_of::<usize>()) as u64,
            )?;
            checked.insert(identity);
            let form = &schema
                .types
                .get(&ty)
                .ok_or_else(|| reject("capture type is absent from canonical authority"))?
                .form;
            let mut append = |child| -> Result<(), ExecutionError> {
                self.charge_allocation(std::mem::size_of::<(TypeObjectDigest, u16)>() as u64)?;
                types.push((child, depth + 1));
                Ok(())
            };
            match form {
                TypeForm::Unit
                | TypeForm::Bool
                | TypeForm::I64
                | TypeForm::Bytes
                | TypeForm::Text
                | TypeForm::StaticText
                | TypeForm::Function { .. }
                | TypeForm::TaskFunction { .. } => {}
                TypeForm::Secret
                | TypeForm::Stream { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::TypeParameter { .. } => {
                    return Err(reject(
                        "capture stores a secret, stream, resource, or unknown type parameter",
                    ));
                }
                TypeForm::List { item } | TypeForm::Option { item } => append(*item)?,
                TypeForm::Map { key, value }
                | TypeForm::Result {
                    ok: key,
                    error: value,
                } => {
                    append(*value)?;
                    append(*key)?;
                }
                TypeForm::StructuralRecord { fields } => {
                    for field in fields.iter().rev() {
                        append(field.ty)?;
                    }
                }
                TypeForm::Named { .. } | TypeForm::Applied { .. } => {
                    if let Some(index) = schema.record_instances.get(&identity).copied() {
                        for argument in schema.records[index].arguments.iter().rev() {
                            append(*argument)?;
                        }
                        for field in schema.records[index].fields.iter().rev() {
                            append(field.ty)?;
                        }
                    } else if let Some(index) = schema.variant_instances.get(&identity).copied() {
                        for argument in schema.variants[index].arguments.iter().rev() {
                            append(*argument)?;
                        }
                        for case in schema.variants[index].cases.iter().rev() {
                            if let Some(payload) = case.payload {
                                append(payload)?;
                            }
                        }
                    } else {
                        return Err(reject("capture has no canonical nominal definition"));
                    }
                }
            }
        }
        Ok(())
    }
}

fn reject(message: impl Into<String>) -> ExecutionError {
    reference_error("normalized_reference_value_admission", message)
}
