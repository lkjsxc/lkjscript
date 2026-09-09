//! Disposable type closure of statically instantiated bytecode calls.
//! Composite substitutions need not already occur as stored type objects. They
//! are derived here without changing type identities or the artifact encoding.

use super::prepare::{
    NormalizedCode, NormalizedEntryPoint, NormalizedFunctionBody, NormalizedInstruction,
    NormalizedProgram,
};
use super::value::FunctionIndex;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{TypeForm, TypeObject, TypeObjectDigest, encode_type_object};
use crate::platform::semantic_id::TypeParameterId;
use std::collections::{BTreeMap, BTreeSet};

type Context = (FunctionIndex, Vec<TypeObjectDigest>);
const MAXIMUM_WORK: usize = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;

fn step(work: &mut usize) -> Result<(), Diagnostic> {
    *work = work
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

pub(super) fn complete(program: &mut NormalizedProgram) -> Result<(), Diagnostic> {
    let mut pending = BTreeSet::new();
    let mut work = 0;
    for (index, function) in program.functions.iter().enumerate() {
        if function.type_parameters.is_empty() {
            step(&mut work)?;
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
        )?;
        calls(
            &test.expected,
            &empty,
            &mut program.types,
            &mut pending,
            &mut work,
        )?;
    }
    for port in program.ports.iter() {
        if let NormalizedEntryPoint::Code(code) | NormalizedEntryPoint::PortExpression(code, _) =
            &port.entry
        {
            calls(code, &empty, &mut program.types, &mut pending, &mut work)?;
        }
    }
    let mut visited = BTreeSet::new();
    while let Some((index, arguments)) = pending.pop_first() {
        step(&mut work)?;
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
            calls(code, &bindings, &mut program.types, &mut pending, &mut work)?;
        }
    }
    program.work.type_objects = program.types.len() as u64;
    program.capture_safe_types = capture_types(program, &mut work)?;
    let mut bytes = program
        .capture_safe_types
        .len()
        .checked_mul(std::mem::size_of::<TypeObjectDigest>() + 3 * std::mem::size_of::<usize>())
        .ok_or_else(missing)?;
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
    program.capture_proof_bytes = bytes;
    Ok(())
}

// Preparation-local proof from compiled layouts and the completed type closure. Every root
// is reconstructed; nominal cycles use a bounded per-root visited set, never a persistent cache.
fn capture_types(
    program: &NormalizedProgram,
    work: &mut usize,
) -> Result<BTreeSet<TypeObjectDigest>, Diagnostic> {
    let mut safe = BTreeSet::new();
    for root in program.types.keys() {
        step(work)?;
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
            visited.insert(ty);
            let mut child = |ty| -> Result<(), Diagnostic> {
                step(work)?;
                pending.push((ty, depth + 1));
                Ok(())
            };
            match &program.types.get(&ty).ok_or_else(missing)?.form {
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
                TypeForm::Named { declaration } => {
                    if let Ok(index) = program
                        .records
                        .binary_search_by_key(declaration, |record| record.declaration)
                    {
                        for field in program.records[index].fields.iter() {
                            child(field.ty)?;
                        }
                    } else if let Ok(index) = program
                        .variants
                        .binary_search_by_key(declaration, |variant| variant.declaration)
                    {
                        for case in program.variants[index].cases.iter() {
                            step(work)?;
                            if let Some(payload) = case.payload {
                                step(work)?;
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
    work: &mut usize,
) -> Result<(), Diagnostic> {
    for instruction in code.instructions.iter() {
        step(work)?;
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
            let arguments = type_arguments
                .iter()
                .map(|ty| substitute(types, *ty, bindings, 0, work))
                .collect::<Result<_, _>>()?;
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
    work: &mut usize,
) -> Result<TypeObjectDigest, Diagnostic> {
    step(work)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(missing());
    }
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
    types.entry(identity).or_insert(object);
    Ok(identity)
}
