//! Disposable type/layout closure of concrete compiled roots and their static substitutions.
//! Composite substitutions need not already occur as stored type objects. They
//! are derived here without changing type identities or the artifact encoding.

use super::prepare::{
    NormalizedCode, NormalizedEntryPoint, NormalizedFunctionBody, NormalizedInstruction,
    NormalizedProgram, NormalizedRecordLayout, NormalizedVariantLayout,
};
use super::value::FunctionIndex;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{TypeForm, TypeObject, TypeObjectDigest, encode_type_object};
use crate::platform::semantic_id::TypeParameterId;
use std::collections::{BTreeMap, BTreeSet};

type Context = (FunctionIndex, Vec<TypeObjectDigest>);
type EffectBindings =
    BTreeMap<crate::platform::kernel::EffectParameterReference, crate::platform::kernel::EffectRow>;
type EffectApplication = (FunctionIndex, Vec<crate::platform::kernel::EffectRow>);
const MAXIMUM_WORK: usize = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
// Finite preparation storage, independent of invocation-lifetime allocation accounting.
const MAXIMUM_METADATA_BYTES: usize = 256 * 1024 * 1024;

pub(super) struct Budget<'a> {
    steps: usize,
    bytes: usize,
    control: &'a crate::platform::execution::ExecutionControl,
}

impl<'a> Budget<'a> {
    pub(super) fn new(control: &'a crate::platform::execution::ExecutionControl) -> Self {
        Self {
            steps: 0,
            bytes: 0,
            control,
        }
    }
    pub(super) fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        reserve_metadata(&mut self.bytes, count, std::mem::size_of::<T>())
    }

    pub(super) fn step(&mut self) -> Result<(), Diagnostic> {
        step(self)
    }

    fn node<T>(&mut self) -> Result<(), Diagnostic> {
        self.reserve::<T>(1)?;
        self.reserve::<usize>(3)
    }

    fn cloned_effect(
        &mut self,
        effect: &crate::platform::kernel::FunctionEffect,
    ) -> Result<(), Diagnostic> {
        if let crate::platform::kernel::FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = effect
        {
            self.reserve::<crate::platform::kernel::RequirementReference>(requirements.len())?;
            self.reserve::<crate::platform::kernel::EffectParameterReference>(
                effect_parameters.len(),
            )?;
        }
        Ok(())
    }

    fn cloned_type(&mut self, object: &TypeObject) -> Result<(), Diagnostic> {
        // Charge variable storage before cloning the canonical expression of a type.
        match &object.form {
            TypeForm::StructuralRecord { fields } => {
                self.reserve::<crate::platform::kernel::StructuralTypeField>(fields.len())?;
                for field in fields {
                    step(self)?;
                    self.reserve::<u8>(field.name.as_str().len())?;
                }
            }
            TypeForm::Applied { arguments, .. } => {
                self.reserve::<TypeObjectDigest>(arguments.len())?
            }
            TypeForm::TaskFunction {
                parameters, effect, ..
            } => {
                self.reserve::<TypeObjectDigest>(parameters.len())?;
                self.reserve::<crate::platform::kernel::RequirementReference>(
                    effect.requirements.len(),
                )?;
                self.reserve::<crate::platform::kernel::EffectParameterReference>(
                    effect.parameters.len(),
                )?;
            }
            TypeForm::Function { parameters, .. } => {
                self.reserve::<TypeObjectDigest>(parameters.len())?
            }
            _ => {}
        }
        Ok(())
    }
}

fn step(work: &mut Budget<'_>) -> Result<(), Diagnostic> {
    work.control
        .check()
        .map_err(|error| Diagnostic::new(DiagnosticClass::Cancelled, error.code, error.message))?;
    work.steps = work
        .steps
        .checked_add(1)
        .filter(|n| *n <= MAXIMUM_WORK)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "normalized_instantiation_work",
                "prepared type closure exceeds its bounded work inventory",
            )
        })?;
    Ok(())
}

fn reserve_metadata(bytes: &mut usize, count: usize, size: usize) -> Result<(), Diagnostic> {
    *bytes = count
        .checked_mul(size)
        .and_then(|added| bytes.checked_add(added))
        .filter(|total| *total <= MAXIMUM_METADATA_BYTES)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "normalized_instantiation_storage",
                "nominal layout metadata exceeds the existing allocation limit",
            )
        })?;
    Ok(())
}

#[cfg(test)]
pub(super) fn complete(program: &mut NormalizedProgram) -> Result<(), Diagnostic> {
    complete_controlled(
        program,
        &crate::platform::execution::ExecutionControl::uncancelled(),
    )
}

pub(super) fn complete_controlled(
    program: &mut NormalizedProgram,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<(), Diagnostic> {
    let mut pending = BTreeSet::new();
    let mut work = Budget::new(control);
    close_effect_applications(program, &mut work)?;
    // Derive dispatch only after each exact effect application and its callees are closed.
    super::prepare::derive_tail_dispatch(
        std::sync::Arc::make_mut(&mut program.functions),
        &mut work,
    )?;
    for (index, function) in program.functions.iter().enumerate() {
        if function.type_parameters.is_empty() && function.effect_parameters.is_empty() {
            step(&mut work)?;
            work.node::<Context>()?;
            pending.insert((
                FunctionIndex(
                    u32::try_from(index).map_err(|_| missing())?,
                    program.value_origin,
                ),
                Vec::new(),
            ));
        }
    }
    let empty = BTreeMap::new();
    for test in program.tests.values() {
        calls(
            &test.actual,
            &empty,
            &mut program.types,
            &mut pending,
            &mut work,
            &program.records,
            &program.variants,
        )?;
        calls(
            &test.expected,
            &empty,
            &mut program.types,
            &mut pending,
            &mut work,
            &program.records,
            &program.variants,
        )?;
    }
    for port in program.ports.iter() {
        if let NormalizedEntryPoint::Code(code) | NormalizedEntryPoint::PortExpression(code, _) =
            &port.entry
        {
            calls(
                code,
                &empty,
                &mut program.types,
                &mut pending,
                &mut work,
                &program.records,
                &program.variants,
            )?;
        }
    }
    let mut visited = BTreeSet::new();
    while let Some((index, arguments)) = pending.pop_first() {
        step(&mut work)?;
        work.node::<Context>()?;
        work.reserve::<TypeObjectDigest>(arguments.len())?;
        if !visited.insert((index, arguments.clone())) {
            continue;
        }
        let function = program
            .functions
            .get(index.0 as usize)
            .ok_or_else(missing)?;
        if function.type_parameters.len() != arguments.len() {
            return Err(missing());
        }
        for _ in &arguments {
            step(&mut work)?;
            work.node::<(TypeParameterId, TypeObjectDigest)>()?;
        }
        let bindings = function
            .type_parameters
            .iter()
            .copied()
            .zip(arguments)
            .collect();
        substitute(&mut program.types, function.result, &bindings, 0, &mut work)?;
        for parameter in function.parameters.iter() {
            substitute(&mut program.types, parameter.ty, &bindings, 0, &mut work)?;
        }
        if let NormalizedFunctionBody::Code(code) = &function.body {
            calls(
                code,
                &bindings,
                &mut program.types,
                &mut pending,
                &mut work,
                &program.records,
                &program.variants,
            )?;
        }
    }
    complete_nominal_layouts(program, &mut work)?;
    program.work.type_objects = program.types.len() as u64;
    program.capture_safe_types = property_types(program, &mut work, Property::Capture)?;
    program.ordinary_types = property_types(program, &mut work, Property::Ordinary)?;
    program.comparable_types = property_types(program, &mut work, Property::Equality)?;
    program.application_free_types = property_types(program, &mut work, Property::NoApplication)?;
    let mut bytes = 0usize;
    for function in program.functions.iter() {
        step(&mut work)?;
        bytes = function
            .type_parameter_constraints
            .len()
            .checked_mul(std::mem::size_of::<
                crate::platform::kernel::TypeParameterConstraints,
            >())
            .and_then(|count| {
                count.checked_add(
                    std::mem::size_of::<
                        std::sync::Arc<[crate::platform::kernel::TypeParameterConstraints]>,
                    >() + 2 * std::mem::size_of::<usize>(),
                )
            })
            .and_then(|count| count.checked_add(bytes))
            .ok_or_else(missing)?;
    }
    work.reserve::<u8>(bytes)?;
    program.capture_proof_bytes = work.bytes;
    program.work.type_derivation_steps = work.steps as u64;
    program.work.type_metadata_bytes = program.capture_proof_bytes as u64;
    Ok(())
}

// Instantiation is keyed by the full canonical type. Template indexes remain stable for
// bytecode field/case selectors; concrete indexes are appended in canonical digest order.
// Generic signatures and intermediate effect substitutions remain useful type metadata,
// but only a fully substituted application has a runtime nominal layout. Including an
// intermediate here would make physical layout indices depend on substitution order.
fn instantiated_type(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    known: &mut BTreeMap<TypeObjectDigest, bool>,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<bool, Diagnostic> {
    step(work)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(missing());
    }
    if let Some(closed) = known.get(&ty) {
        return Ok(*closed);
    }
    let object = types.get(&ty).ok_or_else(missing)?;
    let mut closed = match &object.form {
        TypeForm::TypeParameter { .. } => false,
        TypeForm::TaskFunction { effect, .. } => effect.is_closed(),
        _ => true,
    };
    work.reserve::<TypeObjectDigest>(object.child_type_count())?;
    for child in object.child_types() {
        closed &= instantiated_type(types, child, known, depth + 1, work)?;
    }
    work.node::<(TypeObjectDigest, bool)>()?;
    known.insert(ty, closed);
    Ok(closed)
}

fn complete_nominal_layouts(
    program: &mut NormalizedProgram,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    use super::value::{RecordLayoutIndex, VariantLayoutIndex};
    // Recomputing a disposable closure never promotes an instance into its template.
    work.reserve::<NormalizedRecordLayout>(program.records.len())?;
    work.reserve::<NormalizedVariantLayout>(program.variants.len())?;
    program.records = program
        .records
        .iter()
        .filter(|layout| layout.arguments.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .into();
    program.variants = program
        .variants
        .iter()
        .filter(|layout| layout.arguments.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .into();
    program.record_instances.clear();
    program.variant_instances.clear();
    for _ in program.records.iter() {
        step(work)?;
        work.node::<(
            crate::platform::kernel::DeclarationReference,
            NormalizedRecordLayout,
        )>()?;
    }
    for _ in program.variants.iter() {
        step(work)?;
        work.node::<(
            crate::platform::kernel::DeclarationReference,
            NormalizedVariantLayout,
        )>()?;
    }
    let record_templates = program
        .records
        .iter()
        .map(|layout| (layout.declaration, layout.clone()))
        .collect::<BTreeMap<_, _>>();
    let variant_templates = program
        .variants
        .iter()
        .map(|layout| (layout.declaration, layout.clone()))
        .collect::<BTreeMap<_, _>>();
    for (index, layout) in program.records.iter().enumerate() {
        step(work)?;
        if layout.type_parameters.is_empty() {
            let object = TypeObject::new(TypeForm::Named {
                declaration: layout.declaration,
            })?;
            let (ty, _) = encode_type_object(&object)?;
            if !program.types.contains_key(&ty) {
                work.node::<(TypeObjectDigest, TypeObject)>()?;
            }
            program.types.entry(ty).or_insert(object);
            work.node::<(TypeObjectDigest, RecordLayoutIndex)>()?;
            program.record_instances.insert(
                ty,
                RecordLayoutIndex(
                    u32::try_from(index).map_err(|_| missing())?,
                    program.value_origin,
                ),
            );
        }
    }
    for (index, layout) in program.variants.iter().enumerate() {
        step(work)?;
        if layout.type_parameters.is_empty() {
            let object = TypeObject::new(TypeForm::Named {
                declaration: layout.declaration,
            })?;
            let (ty, _) = encode_type_object(&object)?;
            if !program.types.contains_key(&ty) {
                work.node::<(TypeObjectDigest, TypeObject)>()?;
            }
            program.types.entry(ty).or_insert(object);
            work.node::<(TypeObjectDigest, VariantLayoutIndex)>()?;
            program.variant_instances.insert(
                ty,
                VariantLayoutIndex(
                    u32::try_from(index).map_err(|_| missing())?,
                    program.value_origin,
                ),
            );
        }
    }
    let mut pending = BTreeSet::new();
    for (ty, object) in &program.types {
        step(work)?;
        if matches!(object.form, TypeForm::Applied { .. }) {
            work.node::<TypeObjectDigest>()?;
            pending.insert(*ty);
        }
    }
    let mut visited = BTreeSet::new();
    let mut instantiated = BTreeMap::new();
    let mut derived_records = BTreeMap::new();
    let mut derived_variants = BTreeMap::new();
    while let Some(ty) = pending.pop_first() {
        step(work)?;
        if visited.contains(&ty) {
            continue;
        }
        work.node::<TypeObjectDigest>()?;
        visited.insert(ty);
        if !instantiated_type(&program.types, ty, &mut instantiated, 0, work)? {
            continue;
        }
        let TypeForm::Applied { arguments, .. } = &program.types.get(&ty).ok_or_else(missing)?.form
        else {
            return Err(missing());
        };
        work.reserve::<TypeObjectDigest>(arguments.len())?;
        for _ in arguments {
            step(work)?;
        }
        let TypeForm::Applied {
            declaration,
            arguments,
        } = program.types.get(&ty).ok_or_else(missing)?.form.clone()
        else {
            return Err(missing());
        };
        work.reserve::<TypeObjectDigest>(arguments.len())?;
        let mut member_roots = arguments.clone();
        if let Some(template) = record_templates.get(&declaration) {
            if arguments.len() != template.type_parameters.len() {
                return Err(missing());
            }
            work.node::<(TypeObjectDigest, NormalizedRecordLayout)>()?;
            work.reserve::<super::prepare::NormalizedRecordField>(template.fields.len())?;
            work.reserve::<TypeObjectDigest>(template.fields.len())?;
            for _ in &arguments {
                step(work)?;
                work.node::<(TypeParameterId, TypeObjectDigest)>()?;
            }
            let bindings = template
                .type_parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied())
                .collect();
            let mut fields = Vec::new();
            for field in template.fields.iter() {
                step(work)?;
                work.reserve::<u8>(field.name.as_str().len())?;
                let mut field = field.clone();
                field.ty = substitute(&mut program.types, field.ty, &bindings, 0, work)?;
                member_roots.push(field.ty);
                fields.push(field);
            }
            derived_records.insert(
                ty,
                NormalizedRecordLayout {
                    fields: fields.into(),
                    arguments: arguments.into(),
                    ..template.clone()
                },
            );
        } else if let Some(template) = variant_templates.get(&declaration) {
            if arguments.len() != template.type_parameters.len() {
                return Err(missing());
            }
            work.node::<(TypeObjectDigest, NormalizedVariantLayout)>()?;
            work.reserve::<super::prepare::NormalizedVariantCase>(template.cases.len())?;
            work.reserve::<TypeObjectDigest>(template.cases.len())?;
            for _ in &arguments {
                step(work)?;
                work.node::<(TypeParameterId, TypeObjectDigest)>()?;
            }
            let bindings = template
                .type_parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied())
                .collect();
            let mut cases = Vec::new();
            for case in template.cases.iter() {
                step(work)?;
                work.reserve::<u8>(case.name.as_str().len())?;
                let mut case = case.clone();
                case.payload = case
                    .payload
                    .map(|ty| substitute(&mut program.types, ty, &bindings, 0, work))
                    .transpose()?;
                member_roots.extend(case.payload);
                cases.push(case);
            }
            derived_variants.insert(
                ty,
                NormalizedVariantLayout {
                    cases: cases.into(),
                    arguments: arguments.into(),
                    ..template.clone()
                },
            );
        } else {
            return Err(missing());
        }
        let mut reached = BTreeSet::new();
        work.reserve::<(TypeObjectDigest, usize)>(member_roots.len())?;
        let mut roots = member_roots
            .into_iter()
            .map(|ty| (ty, 0usize))
            .collect::<Vec<_>>();
        while let Some((child, depth)) = roots.pop() {
            step(work)?;
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                return Err(missing());
            }
            if reached.contains(&child) {
                continue;
            }
            work.node::<TypeObjectDigest>()?;
            reached.insert(child);
            let object = program.types.get(&child).ok_or_else(missing)?;
            if matches!(object.form, TypeForm::Applied { .. })
                && !visited.contains(&child)
                && !pending.contains(&child)
            {
                work.node::<TypeObjectDigest>()?;
                pending.insert(child);
            }
            work.cloned_type(object)?;
            for child in object.child_types() {
                step(work)?;
                work.reserve::<(TypeObjectDigest, usize)>(1)?;
                roots.push((child, depth + 1));
            }
        }
    }
    work.reserve::<NormalizedRecordLayout>(program.records.len())?;
    let mut records = program.records.to_vec();
    for (ty, layout) in derived_records {
        step(work)?;
        work.node::<(TypeObjectDigest, RecordLayoutIndex)>()?;
        work.reserve::<NormalizedRecordLayout>(1)?;
        program.record_instances.insert(
            ty,
            RecordLayoutIndex(
                u32::try_from(records.len()).map_err(|_| missing())?,
                program.value_origin,
            ),
        );
        records.push(layout);
    }
    work.reserve::<NormalizedVariantLayout>(program.variants.len())?;
    let mut variants = program.variants.to_vec();
    for (ty, layout) in derived_variants {
        step(work)?;
        work.node::<(TypeObjectDigest, VariantLayoutIndex)>()?;
        work.reserve::<NormalizedVariantLayout>(1)?;
        program.variant_instances.insert(
            ty,
            VariantLayoutIndex(
                u32::try_from(variants.len()).map_err(|_| missing())?,
                program.value_origin,
            ),
        );
        variants.push(layout);
    }
    work.reserve::<bool>(variants.len())?;
    let mut affine = program.affine_variants.to_vec();
    affine.resize(variants.len(), false);
    program.affine_variants = affine.into();
    program.work.record_layouts = records.len() as u64;
    program.work.variant_layouts = variants.len() as u64;
    program.records = records.into();
    program.variants = variants.into();
    Ok(())
}

// Preparation-local greatest fixed point. Disqualifying leaves propagate through reverse
// dependencies, including every argument/member of a cyclic component. No persistent cache.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Property {
    Capture,
    Ordinary,
    Equality,
    NoApplication,
}

fn property_types(
    program: &NormalizedProgram,
    work: &mut Budget<'_>,
    property: Property,
) -> Result<BTreeSet<TypeObjectDigest>, Diagnostic> {
    let mut safe = BTreeSet::new();
    let mut reverse: BTreeMap<TypeObjectDigest, Vec<TypeObjectDigest>> = BTreeMap::new();
    let mut rejected = Vec::new();
    for (root, object) in &program.types {
        step(work)?;
        work.node::<TypeObjectDigest>()?;
        safe.insert(*root);
        let mut admitted = true;
        let mut child = |ty| -> Result<(), Diagnostic> {
            step(work)?;
            if !program.types.contains_key(&ty) {
                return Err(missing());
            }
            if !reverse.contains_key(&ty) {
                work.node::<(TypeObjectDigest, Vec<TypeObjectDigest>)>()?;
            }
            work.reserve::<TypeObjectDigest>(1)?;
            reverse.entry(ty).or_default().push(*root);
            Ok(())
        };
        match &object.form {
            TypeForm::Applied { .. } if property == Property::NoApplication => {
                admitted = false;
            }
            TypeForm::Secret
            | TypeForm::Stream { .. }
            | TypeForm::CapabilityResource { .. }
            | TypeForm::TypeParameter { .. }
                if property == Property::NoApplication => {}
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText => {}
            TypeForm::Function { .. } | TypeForm::TaskFunction { .. }
                if property != Property::Equality => {}
            TypeForm::Secret if property == Property::Ordinary => {}
            TypeForm::Function { .. }
            | TypeForm::TaskFunction { .. }
            | TypeForm::Secret
            | TypeForm::Stream { .. }
            | TypeForm::CapabilityResource { .. }
            | TypeForm::TypeParameter { .. } => {
                admitted = false;
            }
            TypeForm::List { item } | TypeForm::Option { item } => child(*item)?,
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                child(*key)?;
                child(*value)?;
            }
            TypeForm::StructuralRecord { fields } => {
                for field in fields {
                    child(field.ty)?;
                }
            }
            TypeForm::Named { .. } | TypeForm::Applied { .. } => {
                if let Some(index) = program
                    .record_instances
                    .get(root)
                    .map(|index| index.0 as usize)
                {
                    for argument in program.records[index].arguments.iter() {
                        child(*argument)?;
                    }
                    for field in program.records[index].fields.iter() {
                        child(field.ty)?;
                    }
                } else if let Some(index) = program
                    .variant_instances
                    .get(root)
                    .map(|index| index.0 as usize)
                {
                    for argument in program.variants[index].arguments.iter() {
                        child(*argument)?;
                    }
                    for case in program.variants[index].cases.iter() {
                        if let Some(payload) = case.payload {
                            child(payload)?;
                        }
                    }
                } else if matches!(object.form, TypeForm::Applied { .. }) {
                    admitted = false;
                } else {
                    return Err(missing());
                }
            }
        }
        if !admitted {
            work.reserve::<TypeObjectDigest>(1)?;
            safe.remove(root);
            rejected.push(*root);
        }
    }
    while let Some(ty) = rejected.pop() {
        step(work)?;
        if let Some(dependents) = reverse.get(&ty) {
            for dependent in dependents {
                step(work)?;
                if safe.remove(dependent) {
                    work.reserve::<TypeObjectDigest>(1)?;
                    rejected.push(*dependent);
                }
            }
        }
    }
    Ok(safe)
}

fn calls(
    code: &NormalizedCode,
    bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    pending: &mut BTreeSet<Context>,
    work: &mut Budget<'_>,
    records: &[NormalizedRecordLayout],
    variants: &[NormalizedVariantLayout],
) -> Result<(), Diagnostic> {
    for instruction in code.instructions.iter() {
        step(work)?;
        let nominal = match instruction {
            NormalizedInstruction::Record {
                layout: Some(layout),
                type_arguments,
                ..
            } => {
                let template = records.get(layout.0 as usize).ok_or_else(missing)?;
                Some((
                    template.declaration,
                    template.type_parameters.len(),
                    type_arguments,
                ))
            }
            NormalizedInstruction::Variant {
                layout,
                type_arguments,
                ..
            } => {
                let template = variants.get(layout.0 as usize).ok_or_else(missing)?;
                Some((
                    template.declaration,
                    template.type_parameters.len(),
                    type_arguments,
                ))
            }
            _ => None,
        };
        if let Some((declaration, arity, arguments)) = nominal {
            if arity != arguments.len() {
                return Err(missing());
            }
            work.reserve::<TypeObjectDigest>(arguments.len())?;
            let arguments = arguments
                .iter()
                .map(|ty| substitute(types, *ty, bindings, 0, work))
                .collect::<Result<Vec<_>, _>>()?;
            let form = if arguments.is_empty() {
                TypeForm::Named { declaration }
            } else {
                TypeForm::Applied {
                    declaration,
                    arguments,
                }
            };
            let object = TypeObject::new(form)?;
            let (ty, _) = encode_type_object(&object)?;
            step(work)?;
            if !types.contains_key(&ty) {
                work.node::<(TypeObjectDigest, TypeObject)>()?;
            }
            types.entry(ty).or_insert(object);
        }
        if let NormalizedInstruction::Call {
            function,
            type_arguments,
            ..
        }
        | NormalizedInstruction::TailCall {
            function,
            type_arguments,
            ..
        }
        | NormalizedInstruction::FunctionValue {
            effect_arguments: _,
            function,
            type_arguments,
        } = instruction
        {
            work.reserve::<TypeObjectDigest>(type_arguments.len())?;
            let arguments = type_arguments
                .iter()
                .map(|ty| substitute(types, *ty, bindings, 0, work))
                .collect::<Result<_, _>>()?;
            work.node::<Context>()?;
            pending.insert((*function, arguments));
        }
    }
    Ok(())
}

fn missing() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "normalized_instantiation_scope",
        "prepared type closure has a missing exact type, function, or substitution",
    )
}

/// Effect applications have a finite exact requirement universe. Ordinary type applications
/// remain under the existing recursive-call contract and the subsequent type worklist.
fn close_effect_applications(
    program: &mut NormalizedProgram,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    use super::prepare::{NormalizedCode, NormalizedEntryPoint, NormalizedFunction};
    use crate::platform::kernel::{EffectParameterReference, EffectRow, FunctionEffect};
    use std::sync::Arc;

    struct Closing<'a, 'b> {
        templates: Arc<[NormalizedFunction]>,
        functions: Vec<NormalizedFunction>,
        instances: BTreeMap<EffectApplication, FunctionIndex>,
        pending: Vec<(EffectApplication, FunctionIndex)>,
        types: &'a mut BTreeMap<TypeObjectDigest, TypeObject>,
        requirements:
            BTreeMap<crate::platform::kernel::RequirementReference, super::value::RequirementIndex>,
        work: &'a mut Budget<'b>,
    }
    impl Closing<'_, '_> {
        fn rows(&mut self, rows: &[EffectRow]) -> Result<(), Diagnostic> {
            self.work.reserve::<EffectRow>(rows.len())?;
            for row in rows {
                step(self.work)?;
                self.work
                    .reserve::<crate::platform::kernel::RequirementReference>(
                        row.requirements.len(),
                    )?;
                self.work
                    .reserve::<EffectParameterReference>(row.parameters.len())?;
                row.validate()?;
            }
            Ok(())
        }
        fn application(
            &mut self,
            target: FunctionIndex,
            rows: Vec<EffectRow>,
        ) -> Result<FunctionIndex, Diagnostic> {
            step(self.work)?;
            let function = self.templates.get(target.0 as usize).ok_or_else(missing)?;
            if function.effect_parameters.len() != rows.len()
                || rows.iter().any(|row| !row.is_closed())
            {
                return Err(missing());
            }
            let key = (target, rows);
            if let Some(index) = self.instances.get(&key) {
                return Ok(*index);
            }
            self.work.node::<(EffectApplication, FunctionIndex)>()?;
            self.work.reserve::<NormalizedFunction>(1)?;
            let index = if function.effect_parameters.is_empty() {
                target
            } else {
                self.work.cloned_effect(&function.effect)?;
                let index = FunctionIndex(
                    u32::try_from(self.functions.len()).map_err(|_| missing())?,
                    target.1,
                );
                self.functions.push(function.clone());
                index
            };
            self.rows(&key.1)?;
            self.work.node::<(EffectApplication, FunctionIndex)>()?;
            self.instances.insert(key.clone(), index);
            self.pending.push((key, index));
            Ok(index)
        }
        fn code(
            &mut self,
            code: &mut NormalizedCode,
            bindings: &EffectBindings,
        ) -> Result<(), Diagnostic> {
            self.work
                .reserve::<NormalizedInstruction>(code.instructions.len())?;
            // Admit variable operand storage before Arc::make_mut can clone it.
            for instruction in code.instructions.iter() {
                step(self.work)?;
                if let NormalizedInstruction::Call {
                    effect_arguments, ..
                }
                | NormalizedInstruction::TailCall {
                    effect_arguments, ..
                }
                | NormalizedInstruction::FunctionValue {
                    effect_arguments, ..
                } = instruction
                {
                    self.rows(effect_arguments)?;
                }
            }
            for instruction in Arc::make_mut(&mut code.instructions) {
                step(self.work)?;
                match instruction {
                    NormalizedInstruction::Call {
                        function,
                        type_arguments,
                        effect_arguments,
                        ..
                    }
                    | NormalizedInstruction::TailCall {
                        function,
                        type_arguments,
                        effect_arguments,
                        ..
                    }
                    | NormalizedInstruction::FunctionValue {
                        function,
                        type_arguments,
                        effect_arguments,
                    } => {
                        self.work.reserve::<EffectRow>(effect_arguments.len())?;
                        let rows = effect_arguments
                            .iter()
                            .map(|row| {
                                row.substitute(bindings, |n| {
                                    self.work
                                        .reserve::<crate::platform::kernel::RequirementReference>(
                                            n,
                                        )?;
                                    for _ in 0..n {
                                        step(self.work)?;
                                    }
                                    Ok(())
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        *function = self.application(*function, rows)?;
                        *effect_arguments = Arc::from([]);
                        self.work
                            .reserve::<TypeObjectDigest>(type_arguments.len())?;
                        for ty in Arc::make_mut(type_arguments) {
                            *ty = substitute_effect_type(self.types, *ty, bindings, 0, self.work)?;
                        }
                    }
                    NormalizedInstruction::Record { type_arguments, .. }
                    | NormalizedInstruction::Variant { type_arguments, .. } => {
                        self.work
                            .reserve::<TypeObjectDigest>(type_arguments.len())?;
                        for ty in Arc::make_mut(type_arguments) {
                            *ty = substitute_effect_type(self.types, *ty, bindings, 0, self.work)?;
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        }
    }
    let templates = Arc::clone(&program.functions);
    work.reserve::<NormalizedFunction>(templates.len())?;
    for function in templates.iter() {
        step(work)?;
        work.cloned_effect(&function.effect)?;
    }
    work.reserve::<(
        crate::platform::kernel::RequirementReference,
        super::value::RequirementIndex,
    )>(program.requirements.len())?;
    let mut closing = Closing {
        functions: templates.to_vec(),
        templates,
        instances: BTreeMap::new(),
        pending: Vec::new(),
        types: &mut program.types,
        requirements: program
            .requirements
            .iter()
            .enumerate()
            .map(|(i, r)| {
                Ok((
                    r.reference,
                    super::value::RequirementIndex(u32::try_from(i).map_err(|_| missing())?),
                ))
            })
            .collect::<Result<_, Diagnostic>>()?,
        work,
    };
    for i in 0..closing.templates.len() {
        if closing.templates[i].effect_parameters.is_empty() {
            closing.application(
                FunctionIndex(
                    u32::try_from(i).map_err(|_| missing())?,
                    program.value_origin,
                ),
                Vec::new(),
            )?;
        }
    }
    let empty = EffectBindings::new();
    for test in program.tests.values_mut() {
        closing.code(&mut test.actual, &empty)?;
        closing.code(&mut test.expected, &empty)?;
    }
    closing
        .work
        .reserve::<super::prepare::NormalizedPort>(program.ports.len())?;
    for port in Arc::make_mut(&mut program.ports) {
        if let NormalizedEntryPoint::Code(code) | NormalizedEntryPoint::PortExpression(code, _) =
            &mut port.entry
        {
            closing.code(code, &empty)?;
        }
    }
    while let Some(((target, rows), index)) = closing.pending.pop() {
        step(closing.work)?;
        closing.work.reserve::<NormalizedFunction>(1)?;
        closing
            .work
            .cloned_effect(&closing.templates[target.0 as usize].effect)?;
        let mut function = closing.templates[target.0 as usize].clone();
        closing
            .work
            .reserve::<(EffectParameterReference, EffectRow)>(rows.len())?;
        closing.rows(&rows)?;
        let bindings = function
            .effect_parameters
            .iter()
            .zip(&rows)
            .map(|(parameter, row)| {
                (
                    EffectParameterReference {
                        package: function.declaration.package,
                        parameter: *parameter,
                    },
                    row.clone(),
                )
            })
            .collect::<EffectBindings>();
        closing.work.cloned_effect(&function.effect)?;
        let row = function.effect.row().substitute(&bindings, |n| {
            closing
                .work
                .reserve::<crate::platform::kernel::RequirementReference>(n)?;
            for _ in 0..n {
                step(closing.work)?;
            }
            Ok(())
        })?;
        closing
            .work
            .reserve::<super::value::RequirementIndex>(row.requirements.len())?;
        function.task_requirements = row
            .requirements
            .iter()
            .map(|r| closing.requirements.get(r).copied().ok_or_else(missing))
            .collect::<Result<Vec<_>, _>>()?
            .into();
        if matches!(function.effect, FunctionEffect::Task { .. }) {
            function.effect = FunctionEffect::Task {
                requirements: row.requirements,
                effect_parameters: Vec::new(),
            };
        }
        function.effect_parameters = Arc::from([]);
        function.effect_arguments = rows.into();
        function.result =
            substitute_effect_type(closing.types, function.result, &bindings, 0, closing.work)?;
        closing
            .work
            .reserve::<super::prepare::NormalizedParameter>(function.parameters.len())?;
        for parameter in Arc::make_mut(&mut function.parameters) {
            parameter.ty =
                substitute_effect_type(closing.types, parameter.ty, &bindings, 0, closing.work)?;
        }
        if let NormalizedFunctionBody::Code(code) = &mut function.body {
            closing.code(code, &bindings)?;
        }
        closing.functions[index.0 as usize] = function;
    }
    program.functions = closing.functions.into();
    Ok(())
}

fn substitute_effect_type(
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    bindings: &EffectBindings,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<TypeObjectDigest, Diagnostic> {
    step(work)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(missing());
    }
    let object = types.get(&ty).ok_or_else(missing)?;
    work.cloned_type(object)?;
    let mut object = object.clone();
    if let TypeForm::TaskFunction { effect, .. } = &mut object.form {
        *effect = effect.substitute(bindings, |n| {
            work.reserve::<crate::platform::kernel::RequirementReference>(n)?;
            for _ in 0..n {
                step(work)?;
            }
            Ok(())
        })?;
    }
    let mut descend = |ty: &mut TypeObjectDigest| -> Result<(), Diagnostic> {
        *ty = substitute_effect_type(types, *ty, bindings, depth + 1, work)?;
        Ok(())
    };
    match &mut object.form {
        TypeForm::Applied { arguments, .. } => {
            for ty in arguments {
                descend(ty)?;
            }
        }
        TypeForm::StructuralRecord { fields } => {
            for field in fields {
                descend(&mut field.ty)?;
            }
        }
        TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
            descend(item)?
        }
        TypeForm::Map { key, value }
        | TypeForm::Result {
            ok: key,
            error: value,
        } => {
            descend(key)?;
            descend(value)?;
        }
        TypeForm::Function { parameters, result }
        | TypeForm::TaskFunction {
            parameters, result, ..
        } => {
            for ty in parameters {
                descend(ty)?;
            }
            descend(result)?;
        }
        _ => {}
    }
    let (digest, _) = encode_type_object(&object)?;
    if !types.contains_key(&digest) {
        work.node::<(TypeObjectDigest, TypeObject)>()?;
    }
    types.entry(digest).or_insert(object);
    Ok(digest)
}

fn substitute(
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<TypeObjectDigest, Diagnostic> {
    step(work)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(missing());
    }
    work.cloned_type(types.get(&ty).ok_or_else(missing)?)?;
    let mut object = types.get(&ty).cloned().ok_or_else(missing)?;
    let mut descend = |ty: &mut TypeObjectDigest| -> Result<(), Diagnostic> {
        *ty = substitute(types, *ty, bindings, depth + 1, work)?;
        Ok(())
    };
    match &mut object.form {
        TypeForm::TypeParameter { parameter } => {
            return bindings.get(parameter).copied().ok_or_else(missing);
        }
        TypeForm::StructuralRecord { fields } => {
            for field in fields {
                descend(&mut field.ty)?;
            }
        }
        TypeForm::Applied { arguments, .. } => {
            for argument in arguments {
                descend(argument)?;
            }
        }
        TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
            descend(item)?
        }
        TypeForm::Map { key, value } => {
            descend(key)?;
            descend(value)?;
        }
        TypeForm::Result { ok, error } => {
            descend(ok)?;
            descend(error)?;
        }
        TypeForm::Function { parameters, result }
        | TypeForm::TaskFunction {
            parameters, result, ..
        } => {
            for parameter in parameters {
                descend(parameter)?;
            }
            descend(result)?;
        }
        _ => {}
    }
    let (identity, _) = encode_type_object(&object)?;
    step(work)?;
    if !types.contains_key(&identity) {
        work.node::<(TypeObjectDigest, TypeObject)>()?;
    }
    types.entry(identity).or_insert(object);
    Ok(identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn derivation_reservations_fit_exactly_and_fail_before_one_over_growth() {
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        let mut budget = Budget {
            steps: MAXIMUM_WORK - 1,
            bytes: 0,
            control: &control,
        };
        step(&mut budget).unwrap();
        assert_eq!(budget.steps, MAXIMUM_WORK);
        assert_eq!(
            step(&mut budget).unwrap_err().code,
            "normalized_instantiation_work"
        );
        let maximum = super::MAXIMUM_METADATA_BYTES;
        let mut bytes = maximum - 32;
        reserve_metadata(&mut bytes, 1, 32).unwrap();
        assert_eq!(bytes, maximum);
        assert_eq!(
            reserve_metadata(&mut bytes, 1, 1).unwrap_err().code,
            "normalized_instantiation_storage"
        );
        assert_eq!(bytes, maximum);
        assert_eq!(
            reserve_metadata(&mut bytes, usize::MAX, 32)
                .unwrap_err()
                .class,
            DiagnosticClass::Resource
        );
    }
}
