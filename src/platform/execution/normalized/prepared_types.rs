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
const MAXIMUM_WORK: usize = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;

struct Budget<'a> {
    steps: usize,
    bytes: usize,
    control: &'a crate::platform::execution::ExecutionControl,
}

impl Budget<'_> {
    fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        reserve_metadata(&mut self.bytes, count, std::mem::size_of::<T>())
    }

    fn node<T>(&mut self) -> Result<(), Diagnostic> {
        self.reserve::<T>(1)?;
        self.reserve::<usize>(3)
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
        .filter(|total| {
            *total as u64 <= super::vm::NormalizedRunPolicy::default().maximum_allocated_bytes
        })
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
    let mut work = Budget {
        steps: 0,
        bytes: 0,
        control,
    };
    for (index, function) in program.functions.iter().enumerate() {
        if function.type_parameters.is_empty() {
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
    let mut derived_records = BTreeMap::new();
    let mut derived_variants = BTreeMap::new();
    while let Some(ty) = pending.pop_first() {
        step(work)?;
        if visited.contains(&ty) {
            continue;
        }
        work.node::<TypeObjectDigest>()?;
        visited.insert(ty);
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

// Preparation-local proof from compiled layouts and the completed type closure. Every root
// is reconstructed; nominal cycles use a bounded per-root visited set, never a persistent cache.
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
    for root in program.types.keys() {
        step(work)?;
        work.reserve::<(TypeObjectDigest, usize)>(1)?;
        let mut pending = vec![(*root, 0usize)];
        let mut visited = BTreeSet::new();
        let mut admitted = true;
        while let Some((ty, depth)) = pending.pop() {
            step(work)?;
            if depth > 256 {
                admitted = false;
                break;
            }
            if visited.contains(&ty) {
                continue;
            }
            step(work)?;
            work.node::<TypeObjectDigest>()?;
            visited.insert(ty);
            let mut child = |ty| -> Result<(), Diagnostic> {
                step(work)?;
                work.reserve::<(TypeObjectDigest, usize)>(1)?;
                pending.push((ty, depth + 1));
                Ok(())
            };
            match &program.types.get(&ty).ok_or_else(missing)?.form {
                TypeForm::Applied { .. } if property == Property::NoApplication => {
                    admitted = false;
                    break;
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
                TypeForm::Function { .. } if property != Property::Equality => {}
                TypeForm::Secret if property == Property::Ordinary => {}
                TypeForm::Function { .. }
                | TypeForm::Secret
                | TypeForm::Stream { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::TypeParameter { .. } => {
                    admitted = false;
                    break;
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
                        .get(&ty)
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
                        .get(&ty)
                        .map(|index| index.0 as usize)
                    {
                        for argument in program.variants[index].arguments.iter() {
                            child(*argument)?;
                        }
                        for case in program.variants[index].cases.iter() {
                            step(work)?;
                            if let Some(payload) = case.payload {
                                step(work)?;
                                work.reserve::<(TypeObjectDigest, usize)>(1)?;
                                pending.push((payload, depth + 1));
                            }
                        }
                    } else {
                        return Err(missing());
                    }
                }
            }
        }
        if admitted {
            step(work)?;
            work.node::<TypeObjectDigest>()?;
            safe.insert(*root);
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
        TypeForm::Function { parameters, result } => {
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
        let maximum =
            super::super::vm::NormalizedRunPolicy::default().maximum_allocated_bytes as usize;
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
