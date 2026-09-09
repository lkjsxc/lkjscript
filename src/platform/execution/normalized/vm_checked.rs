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
    Port,
    Binding(usize),
    Retained(usize),
    Direct,
    Variant,
}

#[derive(Debug)]
pub(super) struct Value {
    raw: NormalizedValue,
    origin: ValueOrigin,
    class: Class,
}

pub(super) type Invocation = (FunctionIndex, Arc<[TypeObjectDigest]>, Vec<Value>);

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
            (Class::Free | Class::Port, ParameterUse::Unrestricted)
                | (Class::Direct, ParameterUse::Borrow)
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
        let target = program.functions.get(function.0 as usize)
            .filter(|_| function.1 == program.value_origin)
            .ok_or_else(|| admission_error("function constructor has a foreign prepared identity; select the exact callable"))?;
        if target.type_parameters.len() != type_arguments.len()
            || type_arguments
                .iter()
                .any(|ty| program.substitute_type(*ty, &BTreeMap::new(), 0).is_none())
        {
            return Err(admission_error(
                "function constructor requires exact resolved type arguments",
            ));
        }
        if target
            .type_parameter_constraints
            .iter()
            .zip(type_arguments.iter())
            .any(|(constraint, ty)| {
                *constraint == crate::platform::kernel::TypeParameterConstraints::CaptureSafe
                    && !program.capture_safe_types.contains(ty)
            })
        {
            return Err(admission_error(
                "function constructor requires capture-safe type arguments",
            ));
        }
        let pure = target.pure_graph
            || matches!(
                target.body,
                super::super::prepare::NormalizedFunctionBody::External(_)
            );
        Ok(Self {
            raw: NormalizedValue::Function {
                function,
                type_arguments,
                bound_arguments: None,
            },
            origin: program.value_origin,
            class: if pure { Class::Free } else { Class::Port },
        })
    }

    /// Prefix children inherit the construction/admission proof; invocation never readmits them.
    pub(super) fn invocation(
        &self,
        program: &NormalizedProgram,
        allow_port: bool,
    ) -> Result<Invocation, ExecutionError> {
        if self.origin != program.value_origin || (!allow_port && self.class != Class::Free) {
            return Err(admission_error(
                "invoke requires a checked pure callable from this preparation",
            ));
        }
        let NormalizedValue::Function {
            function,
            type_arguments,
            bound_arguments,
        } = &self.raw
        else {
            return Err(type_error("invoke callee is not a function"));
        };
        let arguments = bound_arguments
            .iter()
            .flat_map(|prefix| prefix.iter())
            .map(|raw| Self {
                raw: raw.clone(),
                origin: self.origin,
                class: Class::Free,
            })
            .collect();
        Ok((*function, Arc::clone(type_arguments), arguments))
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
        maximum_length: u64,
        reserve: &mut impl FnMut(super::super::list::Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        Self::free_children(program, &children, work)?;
        reserve(super::super::list::Charge {
            slots: 0,
            bytes: children
                .len()
                .checked_mul(std::mem::size_of::<NormalizedValue>())
                .ok_or_else(|| {
                    resource_error(
                        "normalized_list_storage",
                        "list construction metadata overflowed",
                    )
                })? as u64,
        })?;
        Ok(Self {
            raw: NormalizedValue::List(super::super::list::List::from_items(
                children.into_iter().map(Self::into_raw).collect(),
                maximum_length,
                reserve,
            )?),
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
        maximum_length: u64,
        reserve: &mut impl FnMut(super::super::list::Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        Self::free_children(program, std::slice::from_ref(&self), work)?;
        Self::free_children(program, std::slice::from_ref(&child), work)?;
        let NormalizedValue::List(values) = &mut self.raw else {
            return Err(type_error("list append received a foreign value"));
        };
        *values = values.append(child.raw, maximum_length, reserve)?;
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

type Bindings = BTreeMap<crate::platform::semantic_id::TypeParameterId, TypeObjectDigest>;
type Visit<'a> = (
    &'a NormalizedValue,
    TypeObjectDigest,
    usize,
    bool,
    Arc<Bindings>,
    bool,
);

impl Admission<'_> {
    pub(super) fn begin_bind(
        &mut self,
        mut callee: Value,
        arguments: usize,
    ) -> Result<Value, ExecutionError> {
        self.control.check()?;
        if callee.class(self.program, self.work)? != Class::Free {
            return Err(admission_error(
                "bind requires a pure callable; task-port values cannot become environments",
            ));
        }
        let NormalizedValue::Function {
            function,
            type_arguments,
            bound_arguments,
        } = callee.raw()
        else {
            return Err(type_error("bind callee is not a function"));
        };
        let target = self
            .program
            .functions
            .get(function.0 as usize)
            .filter(|_| function.1 == self.program.value_origin)
            .ok_or_else(|| admission_error("bind target belongs to another prepared program"))?;
        if !(target.pure_graph
            || matches!(
                target.body,
                super::super::prepare::NormalizedFunctionBody::External(_)
            ))
            || !target.task_requirements.is_empty()
            || target
                .parameters
                .iter()
                .any(|parameter| parameter.resource_requirement.is_some())
            || type_arguments.len() != target.type_parameters.len()
        {
            return Err(admission_error("bind target is not an exact pure callable"));
        }
        let existing = bound_arguments.as_deref().map_or(&[][..], Vec::as_slice);
        let count = existing
            .len()
            .checked_add(arguments)
            .filter(|count| {
                *count <= target.parameters.len()
                    && *count <= crate::platform::kernel::contract::MAXIMUM_CHILDREN
            })
            .ok_or_else(|| {
                admission_error("bound prefix exceeds remaining arity or the capture-slot bound")
            })?;
        if arguments != 0 {
            self.collection(count)?;
            self.allocate(
                (std::mem::size_of::<Vec<NormalizedValue>>() + 2 * std::mem::size_of::<usize>())
                    as u64,
            )?;
        }
        callee.class = Class::Binding(arguments);
        Ok(callee)
    }

    pub(super) fn capture(
        &mut self,
        callee: &Value,
        mut argument: Value,
        index: usize,
    ) -> Result<Value, ExecutionError> {
        self.control.check()?;
        if !matches!(callee.class(self.program, self.work)?, Class::Binding(count) if index < count)
            || argument.class(self.program, self.work)? != Class::Free
        {
            return Err(admission_error(
                "capture requires its exact in-progress bind and an ordinary value",
            ));
        }
        let NormalizedValue::Function {
            function,
            type_arguments,
            bound_arguments,
        } = callee.raw()
        else {
            return Err(type_error("capture has no callable target"));
        };
        let target = self
            .program
            .functions
            .get(function.0 as usize)
            .ok_or_else(|| admission_error("capture target is absent"))?;
        let parameter = index
            .checked_add(bound_arguments.as_ref().map_or(0, |prefix| prefix.len()))
            .and_then(|index| target.parameters.get(index))
            .ok_or_else(|| admission_error("capture index exceeds remaining arity"))?;
        self.binding_storage(type_arguments.len())?;
        let bindings = Arc::new(
            target
                .type_parameters
                .iter()
                .copied()
                .zip(type_arguments.iter().copied())
                .collect::<Bindings>(),
        );
        self.allocate(std::mem::size_of::<Visit<'_>>() as u64)?;
        self.inspect_pending(
            vec![(argument.raw(), parameter.ty, 1, false, bindings, true)],
            None,
            false,
        )?;
        self.control.check()?;
        argument.class = Class::Retained(index);
        Ok(argument)
    }

    pub(super) fn bind(
        &mut self,
        mut callee: Value,
        arguments: Vec<Value>,
    ) -> Result<Value, ExecutionError> {
        self.control.check()?;
        if callee.class(self.program, self.work)? != Class::Binding(arguments.len()) {
            return Err(admission_error(
                "bind completion is missing its exact prepared capture count",
            ));
        }
        for (index, argument) in arguments.iter().enumerate() {
            if argument.class(self.program, self.work)? != Class::Retained(index) {
                return Err(admission_error(
                    "bind completion contains an unchecked or reordered capture",
                ));
            }
        }
        if arguments.is_empty() {
            callee.class = Class::Free;
            return Ok(callee);
        }
        let NormalizedValue::Function {
            function,
            type_arguments,
            bound_arguments,
        } = callee.raw()
        else {
            return Err(type_error("bind completion has no callable"));
        };
        let existing = bound_arguments.as_deref().map_or(&[][..], Vec::as_slice);
        let count = existing
            .len()
            .checked_add(arguments.len())
            .filter(|count| *count <= crate::platform::kernel::contract::MAXIMUM_CHILDREN)
            .ok_or_else(|| admission_error("bind completion exceeds the checked prefix bound"))?;
        let mut prefix = Vec::with_capacity(count);
        prefix.extend(existing.iter().cloned());
        prefix.extend(arguments.into_iter().map(Value::into_raw));
        self.control.check()?;
        Ok(Value {
            raw: NormalizedValue::Function {
                function: *function,
                type_arguments: Arc::clone(type_arguments),
                bound_arguments: Some(Arc::new(prefix)),
            },
            origin: self.program.value_origin,
            class: Class::Free,
        })
    }

    pub(super) fn require_capture_type(
        &mut self,
        ty: TypeObjectDigest,
        bindings: &Bindings,
        checked: &mut std::collections::BTreeSet<TypeObjectDigest>,
    ) -> Result<(), ExecutionError> {
        let program = self.program;
        let control = self.control;
        self.allocate(std::mem::size_of::<(TypeObjectDigest, usize)>() as u64)?;
        let mut pending = vec![(ty, 0_usize)];
        while let Some((ty, depth)) = pending.pop() {
            self.control.check()?;
            self.work.capture_admission_nodes = self.work.capture_admission_nodes.saturating_add(1);
            if depth > 256 {
                return Err(resource_error(
                    "normalized_value_depth",
                    "capture type exceeds depth 256",
                ));
            }
            let ty = self.parameter(ty, bindings)?;
            let identity = self
                .type_identity(ty, bindings, 0)
                .ok_or_else(|| admission_error("capture type is unresolved or foreign"))?;
            if checked.contains(&identity) {
                continue;
            }
            self.allocate(
                (std::mem::size_of::<TypeObjectDigest>() + 3 * std::mem::size_of::<usize>()) as u64,
            )?;
            checked.insert(identity);
            let object = self.program.types.get(&ty).ok_or_else(|| {
                admission_error("capture type is absent from the prepared program")
            })?;
            let mut append = |child| -> Result<(), ExecutionError> {
                self.control.check()?;
                self.allocate(std::mem::size_of::<(TypeObjectDigest, usize)>() as u64)?;
                pending.push((child, depth + 1));
                Ok(())
            };
            match &object.form {
                TypeForm::Unit
                | TypeForm::Bool
                | TypeForm::I64
                | TypeForm::Bytes
                | TypeForm::Text
                | TypeForm::StaticText
                | TypeForm::Function { .. } => {}
                TypeForm::Secret
                | TypeForm::Stream { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::TypeParameter { .. } => {
                    return Err(admission_error(
                        "capture type contains a secret, stream, resource, or unresolved stored parameter",
                    ));
                }
                TypeForm::Named { declaration } => {
                    if let Ok(index) = program
                        .records
                        .binary_search_by_key(declaration, |record| record.declaration)
                    {
                        let fields = &program.records[index].fields;
                        for field in fields.iter().rev() {
                            append(field.ty)?;
                        }
                    } else if let Ok(index) = program
                        .variants
                        .binary_search_by_key(declaration, |variant| variant.declaration)
                    {
                        let cases = &program.variants[index].cases;
                        for case in cases.iter().rev() {
                            control.check()?;
                            if let Some(payload) = case.payload {
                                append(payload)?;
                            }
                        }
                    } else {
                        return Err(admission_error("capture has no exact nominal layout"));
                    }
                }
                TypeForm::StructuralRecord { fields } => {
                    for field in fields.iter().rev() {
                        append(field.ty)?;
                    }
                }
                TypeForm::List { item } | TypeForm::Option { item } => append(*item)?,
                TypeForm::Map { key, value } => {
                    append(*value)?;
                    append(*key)?;
                }
                TypeForm::Result { ok, error } => {
                    append(*error)?;
                    append(*ok)?;
                }
            }
        }
        Ok(())
    }

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
        self.allocate(std::mem::size_of::<Visit<'_>>() as u64)?;
        self.binding_storage(self.substitutions.len())?;
        let bindings = Arc::new(self.substitutions.clone());
        self.inspect_pending(vec![(raw, ty, 0, true, bindings, false)], authority, input)
    }

    fn inspect_pending(
        &mut self,
        mut pending: Vec<Visit<'_>>,
        authority: Option<RequirementReference>,
        input: bool,
    ) -> Result<Class, ExecutionError> {
        let mut root_class = None;
        let mut capture_types = std::collections::BTreeSet::new();
        while let Some((value, ty, depth, affine_allowed, bindings, capture)) = pending.pop() {
            self.control.check()?;
            if capture {
                self.work.capture_admission_nodes =
                    self.work.capture_admission_nodes.saturating_add(1);
            } else if input {
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
            if root_class.is_none() {
                root_class = Some(class);
            }
            if class != Class::Free && !affine_allowed {
                return Err(admission_error(
                    "raw aggregate contains affine authority; remove the nested capability owner",
                ));
            }
            let ty = self.parameter(ty, &bindings)?;
            if capture {
                self.require_capture_type(ty, &bindings, &mut capture_types)?;
                if !matches!(class, Class::Free) || matches!(value, NormalizedValue::Resource(_)) {
                    return Err(admission_error(
                        "bound environment contains runtime authority; capture only ordinary immutable data and pure callables",
                    ));
                }
            }
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
                    self.allocate(values.metadata_bytes()?)?;
                    for value in values.iter() {
                        self.control.check()?;
                        children.push((value, *item, false));
                    }
                }
                (NormalizedValue::Option(value), TypeForm::Option { item }) => {
                    if let Some(value) = value {
                        self.collection(1)?;
                        children.push((value.as_ref(), *item, false));
                    }
                }
                (NormalizedValue::Result { success, value }, TypeForm::Result { ok, error }) => {
                    self.collection(1)?;
                    children.push((value.as_ref(), if *success { *ok } else { *error }, false));
                }
                (NormalizedValue::Map(values), TypeForm::Map { key, value: item }) => {
                    self.collection(values.len())?;
                    let key_type = self
                        .program
                        .types
                        .get(&self.parameter(*key, &bindings)?)
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
                        bound_arguments,
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
                        || bound_arguments
                            .as_ref()
                            .is_some_and(|prefix| prefix.is_empty())
                        || bound_arguments.as_ref().map_or(0, |prefix| prefix.len())
                            > crate::platform::kernel::contract::MAXIMUM_CHILDREN
                        || callable
                            .parameters
                            .len()
                            .checked_sub(bound_arguments.as_ref().map_or(0, |prefix| prefix.len()))
                            != Some(parameters.len())
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
                    for (constraint, ty) in callable
                        .type_parameter_constraints
                        .iter()
                        .zip(type_arguments.iter())
                    {
                        if *constraint
                            == crate::platform::kernel::TypeParameterConstraints::CaptureSafe
                        {
                            self.require_capture_type(
                                *ty,
                                &BTreeMap::new(),
                                &mut std::collections::BTreeSet::new(),
                            )?;
                        }
                    }
                    self.binding_storage(type_arguments.len())?;
                    let substitutions = Arc::new(
                        callable
                            .type_parameters
                            .iter()
                            .copied()
                            .zip(type_arguments.iter().copied())
                            .collect::<BTreeMap<_, _>>(),
                    );
                    let prefix_len = bound_arguments.as_ref().map_or(0, |prefix| prefix.len());
                    for (actual, expected) in
                        callable.parameters.iter().skip(prefix_len).zip(parameters)
                    {
                        if self.type_identity(actual.ty, &substitutions, 0).is_none()
                            || self.type_identity(actual.ty, &substitutions, 0)
                                != self.type_identity(*expected, &bindings, 0)
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
                            != self.type_identity(*result, &bindings, 0)
                    {
                        return Err(admission_error(
                            "raw callback result disagrees with the exact type; instantiate the declared callable",
                        ));
                    }
                    if let Some(prefix) = bound_arguments {
                        self.collection(prefix.len())?;
                        self.allocate(
                            (std::mem::size_of::<Vec<NormalizedValue>>()
                                + 2 * std::mem::size_of::<usize>())
                                as u64,
                        )?;
                        for (child, parameter) in
                            prefix.iter().zip(callable.parameters.iter()).rev()
                        {
                            pending.push((
                                child,
                                parameter.ty,
                                depth + 1,
                                false,
                                Arc::clone(&substitutions),
                                true,
                            ));
                        }
                    }
                }
                _ => {
                    return Err(admission_error(
                        "raw value disagrees with its exact type or authority; supply a value for the selected boundary",
                    ));
                }
            }
            for (child, ty, allowed) in children.into_iter().rev() {
                pending.push((
                    child,
                    ty,
                    depth + 1,
                    allowed,
                    Arc::clone(&bindings),
                    capture,
                ));
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

    fn parameter(
        &self,
        ty: TypeObjectDigest,
        bindings: &Bindings,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        if let Some(TypeForm::TypeParameter { parameter }) =
            self.program.types.get(&ty).map(|object| &object.form)
        {
            bindings.get(parameter).copied().filter(|ty| self.program.types.contains_key(ty))
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

    fn binding_storage(&mut self, count: usize) -> Result<(), ExecutionError> {
        // Account the Arc/map headers and bounded map-node storage before collecting.
        let unit = std::mem::size_of::<(
            crate::platform::semantic_id::TypeParameterId,
            TypeObjectDigest,
        )>() + 3 * std::mem::size_of::<usize>();
        let bytes = count
            .checked_mul(unit)
            .and_then(|bytes| {
                bytes
                    .checked_add(std::mem::size_of::<Bindings>() + 2 * std::mem::size_of::<usize>())
            })
            .ok_or_else(|| {
                resource_error(
                    "normalized_allocation",
                    "binding admission metadata overflowed",
                )
            })?;
        self.allocate(bytes as u64)
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
            + std::mem::size_of::<Visit<'_>>();
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
