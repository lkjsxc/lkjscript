//! Production-only construction and admission of immutable execution values.
//! No caller can attach an affine classification to a raw value.

use super::super::prepare::{NormalizedFieldSelector, NormalizedProgram};
use super::super::resource::NormalizedResourceScope;
use super::super::value::{
    FunctionIndex, NormalizedMapKey, NormalizedRecord, NormalizedValue, RecordLayoutIndex,
    ValueOrigin, ValueWork, VariantLayoutIndex,
};
use super::{resource_error, runtime_error, type_error};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{
    Name, ParameterUse, RequirementReference, TypeForm, TypeObjectDigest,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Class {
    Free,
    Direct,
    Variant,
}

#[derive(Debug)]
pub(super) struct Value {
    raw: NormalizedValue,
    origin: ValueOrigin,
    class: Class,
}

impl Value {
    pub(super) fn raw(&self) -> &NormalizedValue {
        &self.raw
    }
    pub(super) fn into_raw(self) -> NormalizedValue {
        self.raw
    }

    pub(super) fn class(
        &self,
        program: &NormalizedProgram,
        work: &mut ValueWork,
    ) -> Result<Class, ExecutionError> {
        #[cfg(test)]
        super::super::value_oracle::forced_descendant_work(&self.raw, work);
        work.classification_decisions = work.classification_decisions.saturating_add(1);
        if self.origin != program.value_origin {
            return Err(admission_error(
                "checked slot belongs to another prepared program; admit the value against the selected program",
            ));
        }
        Ok(self.class)
    }

    pub(super) fn duplicate(&self, use_mode: ParameterUse) -> Result<Self, ExecutionError> {
        if !matches!(
            (self.class, use_mode),
            (Class::Free, ParameterUse::Unrestricted) | (Class::Direct, ParameterUse::Borrow)
        ) {
            return Err(runtime_error(
                "normalized_local_resource_use",
                "affine owner requires an exact borrow or consume; ordinary duplication is forbidden",
            ));
        }
        Ok(Self {
            raw: self.raw.clone(),
            origin: self.origin,
            class: self.class,
        })
    }

    pub(super) fn scalar(
        program: &NormalizedProgram,
        raw: NormalizedValue,
    ) -> Result<Self, ExecutionError> {
        if !matches!(
            raw,
            NormalizedValue::Unit
                | NormalizedValue::Bool(_)
                | NormalizedValue::I64(_)
                | NormalizedValue::Bytes(_)
                | NormalizedValue::Text(_)
                | NormalizedValue::StaticText(_)
        ) {
            return Err(admission_error(
                "scalar construction received an aggregate or runtime authority; use its checked constructor",
            ));
        }
        Ok(Self {
            raw,
            origin: program.value_origin,
            class: Class::Free,
        })
    }

    pub(super) fn function(
        program: &NormalizedProgram,
        function: FunctionIndex,
        type_arguments: Arc<[TypeObjectDigest]>,
    ) -> Result<Self, ExecutionError> {
        if function.1 != program.value_origin
            || program.functions.get(function.0 as usize).is_none()
        {
            return Err(admission_error(
                "function constructor has a foreign prepared identity; select the exact callable",
            ));
        }
        Ok(Self {
            raw: NormalizedValue::Function {
                function,
                type_arguments,
            },
            origin: program.value_origin,
            class: Class::Free,
        })
    }

    fn free_children(
        program: &NormalizedProgram,
        children: &[Self],
        work: &mut ValueWork,
    ) -> Result<(), ExecutionError> {
        for child in children {
            work.constructor_child_visits = work.constructor_child_visits.saturating_add(1);
            if child.class(program, work)? != Class::Free {
                return Err(admission_error(
                    "aggregate construction contains affine authority; transfer the direct owner only through its exact operation",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn list(
        program: &NormalizedProgram,
        children: Vec<Self>,
        work: &mut ValueWork,
    ) -> Result<Self, ExecutionError> {
        Self::free_children(program, &children, work)?;
        Ok(Self {
            raw: NormalizedValue::List(Arc::new(
                children.into_iter().map(Self::into_raw).collect(),
            )),
            origin: program.value_origin,
            class: Class::Free,
        })
    }

    pub(super) fn record(
        program: &NormalizedProgram,
        layout: Option<RecordLayoutIndex>,
        fields: Vec<(Name, Self)>,
        work: &mut ValueWork,
    ) -> Result<Self, ExecutionError> {
        for (_, child) in &fields {
            Self::free_children(program, std::slice::from_ref(child), work)?;
        }
        let record = if let Some(layout) = layout {
            if layout.1 != program.value_origin
                || program
                    .records
                    .get(layout.0 as usize)
                    .is_none_or(|record| record.fields.len() != fields.len())
            {
                return Err(admission_error(
                    "nominal record constructor has a foreign layout; rebuild from accepted meaning",
                ));
            }
            NormalizedRecord::Nominal {
                layout,
                fields: Arc::new(fields.into_iter().map(|(_, value)| value.raw).collect()),
            }
        } else {
            if fields.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
                return Err(admission_error(
                    "structural record fields are not unique and canonical; use ordered field identities",
                ));
            }
            NormalizedRecord::Structural {
                fields: Arc::new(
                    fields
                        .into_iter()
                        .map(|(name, value)| (name, value.raw))
                        .collect(),
                ),
            }
        };
        Ok(Self {
            raw: NormalizedValue::Record(record),
            origin: program.value_origin,
            class: Class::Free,
        })
    }

    pub(super) fn variant(
        program: &NormalizedProgram,
        layout: VariantLayoutIndex,
        case: u32,
        payload: Option<Self>,
        work: &mut ValueWork,
    ) -> Result<Self, ExecutionError> {
        let selected = program.variants.get(layout.0 as usize).and_then(|variant| variant.cases.get(case as usize))
            .filter(|_| layout.1 == program.value_origin).ok_or_else(|| admission_error("variant constructor has a foreign layout or case; select the exact canonical case"))?;
        if selected.payload.is_some() != payload.is_some() {
            return Err(admission_error(
                "variant payload presence disagrees with its canonical case; supply the exact payload",
            ));
        }
        if let Some(payload) = &payload {
            work.constructor_child_visits = work.constructor_child_visits.saturating_add(1);
            let direct = selected
                .payload
                .and_then(|ty| program.types.get(&ty))
                .is_some_and(|object| matches!(object.form, TypeForm::CapabilityResource { .. }));
            if payload.class(program, work)? != if direct { Class::Direct } else { Class::Free } {
                return Err(admission_error(
                    "variant payload has forbidden affine containment; consume the exact direct owner",
                ));
            }
        }
        let class = if program.affine_variants[layout.0 as usize] {
            Class::Variant
        } else {
            Class::Free
        };
        Ok(Self {
            raw: NormalizedValue::Variant {
                layout,
                case,
                payload: payload.map(|value| Box::new(value.raw)),
            },
            origin: program.value_origin,
            class,
        })
    }

    pub(super) fn option(
        program: &NormalizedProgram,
        child: Option<Self>,
        work: &mut ValueWork,
    ) -> Result<Self, ExecutionError> {
        if let Some(child) = &child {
            Self::free_children(program, std::slice::from_ref(child), work)?;
        }
        Ok(Self {
            raw: NormalizedValue::Option(child.map(|child| Box::new(child.raw))),
            origin: program.value_origin,
            class: Class::Free,
        })
    }

    pub(super) fn map(
        program: &NormalizedProgram,
        entries: BTreeMap<NormalizedMapKey, Self>,
        work: &mut ValueWork,
    ) -> Result<Self, ExecutionError> {
        for child in entries.values() {
            Self::free_children(program, std::slice::from_ref(child), work)?;
        }
        Ok(Self {
            raw: NormalizedValue::Map(Arc::new(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, value.raw))
                    .collect(),
            )),
            origin: program.value_origin,
            class: Class::Free,
        })
    }

    // A free parent proves that every descendant is free. These methods expose only actual
    // children, not a caller-selected raw value or asserted classification.
    pub(super) fn field(self, selector: &NormalizedFieldSelector) -> Result<Self, ExecutionError> {
        let raw = super::select_field(self.raw, selector)?;
        Ok(Self {
            raw,
            origin: self.origin,
            class: Class::Free,
        })
    }

    pub(super) fn split_variant(
        self,
        program: &NormalizedProgram,
    ) -> Result<(VariantLayoutIndex, u32, Option<Self>), ExecutionError> {
        let NormalizedValue::Variant {
            layout,
            case,
            payload,
        } = self.raw
        else {
            return Err(type_error("match value is not a variant"));
        };
        let direct = program
            .variants
            .get(layout.0 as usize)
            .and_then(|v| v.cases.get(case as usize))
            .and_then(|case| case.payload)
            .and_then(|ty| program.types.get(&ty))
            .is_some_and(|ty| matches!(ty.form, TypeForm::CapabilityResource { .. }));
        Ok((
            layout,
            case,
            payload.map(|payload| Self {
                raw: *payload,
                origin: self.origin,
                class: if direct { Class::Direct } else { Class::Free },
            }),
        ))
    }

    pub(super) fn append(
        mut self,
        program: &NormalizedProgram,
        child: Self,
        work: &mut ValueWork,
    ) -> Result<Self, ExecutionError> {
        Self::free_children(program, std::slice::from_ref(&self), work)?;
        Self::free_children(program, std::slice::from_ref(&child), work)?;
        let NormalizedValue::List(values) = &mut self.raw else {
            return Err(type_error("list append received a foreign value"));
        };
        let mut output = Vec::with_capacity(values.len().saturating_add(1));
        output.extend(values.iter().cloned());
        output.push(child.raw);
        *values = Arc::new(output);
        Ok(self)
    }

    pub(super) fn edit_map(
        mut self,
        program: &NormalizedProgram,
        key: NormalizedMapKey,
        child: Option<Self>,
        work: &mut ValueWork,
    ) -> Result<Self, ExecutionError> {
        Self::free_children(program, std::slice::from_ref(&self), work)?;
        if let Some(child) = &child {
            Self::free_children(program, std::slice::from_ref(child), work)?;
        }
        let NormalizedValue::Map(values) = &mut self.raw else {
            return Err(type_error("map update received a foreign value"));
        };
        let mut output = values.as_ref().clone();
        if let Some(child) = child {
            output.insert(key, child.raw);
        } else {
            output.remove(&key);
        }
        *values = Arc::new(output);
        Ok(self)
    }

    pub(super) fn list_get(&self, index: usize) -> Result<Self, ExecutionError> {
        let NormalizedValue::List(values) = &self.raw else {
            return Err(type_error("list lookup received a foreign value"));
        };
        let raw = values.get(index).cloned().ok_or_else(|| {
            super::trap_error("normalized_list_index", "list index is out of bounds")
        })?;
        Ok(Self {
            raw,
            origin: self.origin,
            class: Class::Free,
        })
    }

    pub(super) fn option_get(&self) -> Result<Option<Self>, ExecutionError> {
        let NormalizedValue::Option(value) = &self.raw else {
            return Err(type_error("option lookup received a foreign value"));
        };
        Ok(value.as_deref().map(|value| Self {
            raw: value.clone(),
            origin: self.origin,
            class: Class::Free,
        }))
    }

    pub(super) fn map_get(&self, key: &NormalizedMapKey) -> Result<Option<Self>, ExecutionError> {
        let NormalizedValue::Map(values) = &self.raw else {
            return Err(type_error("map lookup received a foreign value"));
        };
        Ok(values.get(key).map(|value| Self {
            raw: value.clone(),
            origin: self.origin,
            class: Class::Free,
        }))
    }
}

pub(super) struct Admission<'a> {
    pub program: &'a NormalizedProgram,
    pub substitutions:
        &'a BTreeMap<crate::platform::semantic_id::TypeParameterId, TypeObjectDigest>,
    pub resources: &'a NormalizedResourceScope,
    pub control: &'a ExecutionControl,
    pub policy: super::NormalizedRunPolicy,
    pub work: &'a mut ValueWork,
    pub allocated: &'a mut u64,
    pub allocation_charges: &'a mut u64,
    pub items: &'a mut u64,
}

impl Admission<'_> {
    pub(super) fn value(
        &mut self,
        raw: NormalizedValue,
        ty: TypeObjectDigest,
        authority: Option<RequirementReference>,
        input: bool,
    ) -> Result<Value, ExecutionError> {
        match self.inspect(&raw, ty, authority, input) {
            Ok(class) => Ok(Value {
                raw,
                origin: self.program.value_origin,
                class,
            }),
            Err(mut error) => {
                super::super::value::release_raw_values(vec![raw]);
                error.message = format!(
                    "{} value admission: {}",
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

    fn inspect(
        &mut self,
        raw: &NormalizedValue,
        ty: TypeObjectDigest,
        authority: Option<RequirementReference>,
        input: bool,
    ) -> Result<Class, ExecutionError> {
        self.control.check()?;
        let mut root_class = None;
        let mut pending = Vec::new();
        self.allocate(
            std::mem::size_of::<(&NormalizedValue, TypeObjectDigest, usize, bool)>() as u64,
        )?;
        pending.push((raw, ty, 0_usize, true));
        while let Some((value, ty, depth, affine_allowed)) = pending.pop() {
            self.control.check()?;
            if input {
                self.work.input_admission_nodes = self.work.input_admission_nodes.saturating_add(1);
            } else {
                self.work.raw_result_admission_nodes =
                    self.work.raw_result_admission_nodes.saturating_add(1);
            }
            if depth > 256 {
                return Err(resource_error(
                    "normalized_value_depth",
                    "value admission exceeds depth 256; reduce the input nesting",
                ));
            }
            let class = self.classify(value)?;
            if depth == 0 {
                root_class = Some(class);
            }
            if class != Class::Free && !affine_allowed {
                return Err(admission_error(
                    "raw aggregate contains affine authority; remove the nested capability owner",
                ));
            }
            let ty = self.parameter(ty)?;
            let object = self.program.types.get(&ty).ok_or_else(|| {
                admission_error(
                    "raw value has an unknown exact type; use the selected prepared program",
                )
            })?;
            let mut children = Vec::new();
            match (value, &object.form) {
                (NormalizedValue::Unit, TypeForm::Unit)
                | (NormalizedValue::Bool(_), TypeForm::Bool)
                | (NormalizedValue::I64(_), TypeForm::I64) => {}
                (NormalizedValue::Text(text), TypeForm::Text)
                | (NormalizedValue::StaticText(text), TypeForm::StaticText) => {
                    self.allocate(text.len() as u64)?
                }
                (NormalizedValue::Bytes(bytes), TypeForm::Bytes) => {
                    self.allocate(bytes.len() as u64)?
                }
                (
                    NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }),
                    TypeForm::Named { declaration },
                ) => {
                    let definition = self.program.records.get(layout.0 as usize).filter(|definition| layout.1 == self.program.value_origin && definition.declaration == *declaration && definition.fields.len() == fields.len())
                        .ok_or_else(|| admission_error("raw nominal record has a foreign identity or shape; use the exact record layout"))?;
                    self.collection(fields.len())?;
                    for (field, definition) in fields.iter().zip(definition.fields.iter()) {
                        children.push((field, definition.ty, false));
                    }
                }
                (
                    NormalizedValue::Record(NormalizedRecord::Structural { fields }),
                    TypeForm::StructuralRecord { fields: expected },
                ) => {
                    if fields.len() != expected.len() {
                        return Err(admission_error(
                            "raw structural record has foreign fields; supply the exact declared shape",
                        ));
                    }
                    self.collection(fields.len())?;
                    for ((name, field), expected) in fields.iter().zip(expected) {
                        if *name != expected.name {
                            return Err(admission_error(
                                "raw structural fields are not canonical; use the exact ordered fields",
                            ));
                        }
                        self.allocate(name.as_str().len() as u64)?;
                        children.push((field, expected.ty, false));
                    }
                }
                (
                    NormalizedValue::Variant {
                        layout,
                        case,
                        payload,
                    },
                    TypeForm::Named { declaration },
                ) => {
                    let selected = self.program.variants.get(layout.0 as usize).filter(|definition| layout.1 == self.program.value_origin && definition.declaration == *declaration)
                        .and_then(|definition| definition.cases.get(*case as usize)).ok_or_else(|| admission_error("raw variant has a foreign identity or case; use the exact nominal layout"))?;
                    match (payload, selected.payload) {
                        (None, None) => {}
                        (Some(value), Some(ty)) => {
                            self.collection(1)?;
                            let direct = self.program.types.get(&ty).is_some_and(|object| {
                                matches!(object.form, TypeForm::CapabilityResource { .. })
                            });
                            children.push((
                                value.as_ref(),
                                ty,
                                affine_allowed && class == Class::Variant && direct,
                            ));
                        }
                        _ => {
                            return Err(admission_error(
                                "raw variant payload presence disagrees with its case; supply the exact payload",
                            ));
                        }
                    }
                }
                (NormalizedValue::List(values), TypeForm::List { item }) => {
                    self.collection(values.len())?;
                    for value in values.iter() {
                        children.push((value, *item, false));
                    }
                }
                (NormalizedValue::Option(value), TypeForm::Option { item }) => {
                    if let Some(value) = value {
                        self.collection(1)?;
                        children.push((value.as_ref(), *item, false));
                    }
                }
                (NormalizedValue::Map(values), TypeForm::Map { key, value: item }) => {
                    self.collection(values.len())?;
                    let key_type =
                        self.program
                            .types
                            .get(&self.parameter(*key)?)
                            .ok_or_else(|| {
                                admission_error(
                                    "raw map has an unknown key type; supply an exact map type",
                                )
                            })?;
                    for (key, value) in values.iter() {
                        self.control.check()?;
                        if !matches!(
                            (key, &key_type.form),
                            (NormalizedMapKey::Bool(_), TypeForm::Bool)
                                | (NormalizedMapKey::I64(_), TypeForm::I64)
                                | (NormalizedMapKey::Bytes(_), TypeForm::Bytes)
                                | (
                                    NormalizedMapKey::Text(_),
                                    TypeForm::Text | TypeForm::StaticText
                                )
                        ) {
                            return Err(admission_error(
                                "raw map has a foreign key type; supply exact typed keys",
                            ));
                        }
                        self.allocate(super::map_key_bytes(key))?;
                        children.push((value, *item, false));
                    }
                }
                (NormalizedValue::Resource(handle), TypeForm::CapabilityResource { interface })
                    if class == Class::Direct && authority.is_some() =>
                {
                    self.resources
                        .validate_admission(*handle, authority, Some(*interface))?;
                }
                (NormalizedValue::Resource(handle), TypeForm::Stream { item })
                    if !handle.is_affine_capability()
                        && self
                            .program
                            .types
                            .get(item)
                            .is_some_and(|ty| matches!(ty.form, TypeForm::Bytes)) =>
                {
                    self.resources.validate_admission(*handle, None, None)?;
                }
                (
                    NormalizedValue::Function {
                        function,
                        type_arguments,
                    },
                    TypeForm::Function { parameters, result },
                ) => {
                    let callable = self.program.functions.get(function.0 as usize).filter(|_| function.1 == self.program.value_origin).ok_or_else(|| admission_error("raw function belongs to another prepared program; select its exact callable"))?;
                    if !(callable.pure_graph
                        || matches!(
                            callable.body,
                            super::super::prepare::NormalizedFunctionBody::External(_)
                        ))
                        || !callable.task_requirements.is_empty()
                        || callable
                            .parameters
                            .iter()
                            .any(|parameter| parameter.resource_requirement.is_some())
                        || callable.type_parameters.len() != type_arguments.len()
                        || callable.parameters.len() != parameters.len()
                    {
                        return Err(admission_error(
                            "raw callback has a foreign signature or effect; pass the exact pure callable",
                        ));
                    }
                    if type_arguments.iter().any(|ty| {
                        self.program
                            .substitute_type(*ty, &BTreeMap::new(), 0)
                            .is_none()
                    }) {
                        return Err(admission_error(
                            "raw callback has a foreign or unresolved type argument; supply exact canonical types",
                        ));
                    }
                    self.allocate(
                        (type_arguments.len()
                            * std::mem::size_of::<(
                                crate::platform::semantic_id::TypeParameterId,
                                TypeObjectDigest,
                            )>()) as u64,
                    )?;
                    let substitutions = callable
                        .type_parameters
                        .iter()
                        .copied()
                        .zip(type_arguments.iter().copied())
                        .collect::<BTreeMap<_, _>>();
                    for (actual, expected) in callable.parameters.iter().zip(parameters) {
                        if self.type_identity(actual.ty, &substitutions, 0).is_none()
                            || self.type_identity(actual.ty, &substitutions, 0)
                                != self.type_identity(*expected, self.substitutions, 0)
                        {
                            return Err(admission_error(
                                "raw callback parameters disagree with the exact type; instantiate the declared callable",
                            ));
                        }
                    }
                    if self
                        .type_identity(callable.result, &substitutions, 0)
                        .is_none()
                        || self.type_identity(callable.result, &substitutions, 0)
                            != self.type_identity(*result, self.substitutions, 0)
                    {
                        return Err(admission_error(
                            "raw callback result disagrees with the exact type; instantiate the declared callable",
                        ));
                    }
                }
                _ => {
                    return Err(admission_error(
                        "raw value disagrees with its exact type or authority; supply a value for the selected boundary",
                    ));
                }
            }
            for (child, ty, allowed) in children.into_iter().rev() {
                pending.push((child, ty, depth + 1, allowed));
            }
        }
        let root_class = root_class.ok_or_else(|| {
            admission_error("raw admission did not visit its root; retry with an intact evaluator")
        })?;
        if root_class != Class::Free && authority.is_none() {
            return Err(admission_error(
                "raw result cannot acquire affine authority; use an authorized exact capability operation",
            ));
        }
        self.allocate(
            (std::mem::size_of::<Value>() - std::mem::size_of::<NormalizedValue>()) as u64,
        )?;
        Ok(root_class)
    }

    fn parameter(&self, ty: TypeObjectDigest) -> Result<TypeObjectDigest, ExecutionError> {
        if let Some(TypeForm::TypeParameter { parameter }) =
            self.program.types.get(&ty).map(|object| &object.form)
        {
            self.substitutions.get(parameter).copied().filter(|ty| self.program.types.contains_key(ty))
                .ok_or_else(|| admission_error("raw value type parameter is unbound; supply the exact declared type arguments"))
        } else {
            Ok(ty)
        }
    }

    fn type_identity(
        &self,
        ty: TypeObjectDigest,
        bindings: &BTreeMap<crate::platform::semantic_id::TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Option<TypeObjectDigest> {
        if depth > 256 {
            return None;
        }
        let descend = |ty| self.type_identity(ty, bindings, depth + 1);
        let form = match &self.program.types.get(&ty)?.form {
            TypeForm::TypeParameter { parameter } => {
                return bindings
                    .get(parameter)
                    .copied()
                    .filter(|ty| self.program.types.contains_key(ty));
            }
            TypeForm::StructuralRecord { fields } => TypeForm::StructuralRecord {
                fields: fields
                    .iter()
                    .map(|field| {
                        Some(crate::platform::kernel::StructuralTypeField {
                            name: field.name.clone(),
                            ty: descend(field.ty)?,
                        })
                    })
                    .collect::<Option<_>>()?,
            },
            TypeForm::List { item } => TypeForm::List {
                item: descend(*item)?,
            },
            TypeForm::Option { item } => TypeForm::Option {
                item: descend(*item)?,
            },
            TypeForm::Stream { item } => TypeForm::Stream {
                item: descend(*item)?,
            },
            TypeForm::Map { key, value } => TypeForm::Map {
                key: descend(*key)?,
                value: descend(*value)?,
            },
            TypeForm::Result { ok, error } => TypeForm::Result {
                ok: descend(*ok)?,
                error: descend(*error)?,
            },
            TypeForm::Function { parameters, result } => TypeForm::Function {
                parameters: parameters
                    .iter()
                    .copied()
                    .map(descend)
                    .collect::<Option<_>>()?,
                result: descend(*result)?,
            },
            form => form.clone(),
        };
        crate::platform::kernel::encode_type_object(
            &crate::platform::kernel::TypeObject::new(form).ok()?,
        )
        .ok()
        .map(|(digest, _)| digest)
    }

    fn classify(&self, value: &NormalizedValue) -> Result<Class, ExecutionError> {
        match value {
            NormalizedValue::Resource(handle) if handle.is_affine_capability() => Ok(Class::Direct),
            NormalizedValue::Variant { layout, .. } => {
                if layout.1 != self.program.value_origin {
                    return Err(admission_error(
                        "raw variant belongs to another prepared program; decode it against the selected program",
                    ));
                }
                self.program
                    .affine_variants
                    .get(layout.0 as usize)
                    .map(|affine| if *affine { Class::Variant } else { Class::Free })
                    .ok_or_else(|| {
                        admission_error(
                            "raw variant layout is absent; use a current nominal identity",
                        )
                    })
            }
            _ => Ok(Class::Free),
        }
    }

    fn allocate(&mut self, bytes: u64) -> Result<(), ExecutionError> {
        let next = self
            .allocated
            .checked_add(bytes)
            .filter(|value| *value <= self.policy.maximum_allocated_bytes)
            .ok_or_else(|| {
                resource_error(
                    "normalized_allocation",
                    "value admission exceeds cumulative allocated bytes; reduce the input",
                )
            })?;
        *self.allocated = next;
        if bytes != 0 {
            *self.allocation_charges = self.allocation_charges.saturating_add(1);
        }
        Ok(())
    }

    fn collection(&mut self, count: usize) -> Result<(), ExecutionError> {
        let next = self
            .items
            .checked_add(count as u64)
            .filter(|value| *value <= self.policy.maximum_collection_items)
            .ok_or_else(|| {
                resource_error(
                    "normalized_collection_items",
                    "value admission exceeds aggregate collection items; reduce the input",
                )
            })?;
        *self.items = next;
        // Raw payload storage and both bounded traversal worklists are charged before growth.
        let unit = std::mem::size_of::<NormalizedValue>()
            + std::mem::size_of::<(&NormalizedValue, TypeObjectDigest, bool)>()
            + std::mem::size_of::<(&NormalizedValue, TypeObjectDigest, usize, bool)>();
        self.allocate((count as u64).checked_mul(unit as u64).ok_or_else(|| {
            resource_error("normalized_allocation", "value admission size overflowed")
        })?)
    }
}

fn admission_error(message: &'static str) -> ExecutionError {
    runtime_error("normalized_value_admission", message)
}

#[cfg(test)]
#[path = "vm_checked_tests.rs"]
mod tests;
