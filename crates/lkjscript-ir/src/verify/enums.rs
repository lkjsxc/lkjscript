use std::collections::HashSet;

use crate::verify::*;
use crate::{EnumMetadata, EnumVariantMetadata, Program, RuntimeLayoutId, SsaType, VariantFieldId};

pub(crate) fn verify_enum_metadata(program: &Program) -> crate::Result<()> {
    let mut enum_ids = HashSet::new();
    let mut names = HashSet::new();
    let mut layouts = HashSet::new();
    let mut variants = HashSet::new();
    let mut fields = HashSet::new();
    for definition in &program.enums {
        if !definition.id.is_resolved()
            || !definition.layout.identity.is_resolved()
            || definition.name.is_empty()
            || !enum_ids.insert(definition.id)
            || !names.insert(definition.name.as_str())
            || !layouts.insert(definition.layout.identity)
            || definition.variants.is_empty()
            || !super::prelude_enums::valid(definition)
        {
            return fail("SSA enum has invalid or duplicate identity/name/layout");
        }
        let mut parameters = HashSet::new();
        if definition
            .type_parameters
            .iter()
            .any(|name| name.is_empty() || !parameters.insert(name.as_str()))
        {
            return fail("SSA enum has invalid type parameters");
        }
        let scope: Vec<_> = definition
            .type_parameters
            .iter()
            .map(String::as_str)
            .collect();
        let mut tags = HashSet::new();
        for (source_order, variant) in definition.variants.iter().enumerate() {
            verify_variant(program, variant, &scope, &mut variants, &mut fields)?;
            if u64::try_from(source_order).ok() != Some(variant.source_order)
                || !tags.insert(variant.physical_tag)
                || usize::try_from(variant.physical_tag)
                    .map_or(true, |tag| tag >= definition.variants.len())
            {
                return fail("SSA enum has malformed physical tags");
            }
        }
        let mut recursive = false;
        for field in definition
            .variants
            .iter()
            .flat_map(|variant| &variant.fields)
        {
            recursive |= contains_enum(&field.ty, definition.id)?;
        }
        if recursive != definition.layout.recursive {
            return fail("SSA enum recursive layout fact is invalid");
        }
    }
    Ok(())
}

fn verify_variant(
    program: &Program,
    variant: &EnumVariantMetadata,
    scope: &[&str],
    variants: &mut HashSet<VariantId>,
    fields: &mut HashSet<VariantFieldId>,
) -> crate::Result<()> {
    if !variant.id.is_resolved() || variant.name.is_empty() || !variants.insert(variant.id) {
        return fail("SSA enum variant has invalid or duplicate identity/name");
    }
    let mut names = HashSet::new();
    for field in &variant.fields {
        if !field.id.is_resolved()
            || field.name.is_empty()
            || !fields.insert(field.id)
            || !names.insert(field.name.as_str())
        {
            return fail("SSA enum field has invalid or duplicate identity/name");
        }
        verify_type(program, &field.ty, scope)?;
        if field.indirect != contains_any_enum(&field.ty) {
            return fail("SSA enum field has invalid storage/layout facts");
        }
    }
    Ok(())
}

pub(crate) fn enum_by_id(program: &Program, id: crate::EnumId) -> crate::Result<&EnumMetadata> {
    program
        .enums
        .iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| crate::IrError::new("missing SSA EnumId"))
}

pub(crate) fn variant_by_id(
    definition: &EnumMetadata,
    id: VariantId,
) -> crate::Result<&EnumVariantMetadata> {
    definition
        .variants
        .iter()
        .find(|variant| variant.id == id)
        .ok_or_else(|| crate::IrError::new("missing SSA VariantId for enum"))
}

pub(crate) fn check_layout(
    definition: &EnumMetadata,
    layout: RuntimeLayoutId,
) -> crate::Result<()> {
    if definition.layout.identity == layout {
        Ok(())
    } else {
        fail("SSA enum runtime layout identity mismatch")
    }
}

pub(crate) fn substitute(
    ty: &SsaType,
    parameters: &[String],
    arguments: &[SsaType],
) -> crate::Result<SsaType> {
    enum Work<'a> {
        Visit(&'a SsaType),
        Enter(&'a [String]),
        Exit(&'a [String]),
        Enum(crate::EnumId, usize),
        List,
        Function {
            type_parameters: &'a [String],
            bounds: &'a [crate::TraitBound],
            witnesses: &'a [crate::MemoryWitnessParameter],
            parameter_count: usize,
        },
    }

    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| crate::IrError::new("SSA enum substitution work allocation failed"))?;
    pending.push(Work::Visit(ty));
    let mut completed = Vec::new();
    let mut bound = std::collections::HashMap::<&str, usize>::new();
    while let Some(item) = pending.pop() {
        match item {
            Work::Visit(ty) => match ty {
                SsaType::TypeParameter(name) => {
                    let substitution = if bound.contains_key(name.as_str()) {
                        None
                    } else {
                        parameters
                            .iter()
                            .position(|parameter| parameter == name)
                            .and_then(|index| arguments.get(index))
                    };
                    let substituted = substitution.cloned().unwrap_or_else(|| ty.clone());
                    completed.try_reserve(1).map_err(|_| {
                        crate::IrError::new("SSA enum substitution result allocation failed")
                    })?;
                    completed.push(substituted);
                }
                SsaType::List(inner) => {
                    pending.try_reserve(2).map_err(|_| {
                        crate::IrError::new("SSA enum substitution work allocation failed")
                    })?;
                    pending.push(Work::List);
                    pending.push(Work::Visit(inner));
                }
                SsaType::Enum {
                    id,
                    arguments: nested,
                } => {
                    let additional = nested.len().checked_add(1).ok_or_else(|| {
                        crate::IrError::new("SSA enum substitution child count overflow")
                    })?;
                    pending.try_reserve(additional).map_err(|_| {
                        crate::IrError::new("SSA enum substitution work allocation failed")
                    })?;
                    pending.push(Work::Enum(*id, nested.len()));
                    pending.extend(nested.iter().rev().map(Work::Visit));
                }
                SsaType::Function(signature) => {
                    let additional =
                        signature.parameters.len().checked_add(5).ok_or_else(|| {
                            crate::IrError::new("SSA enum substitution child count overflow")
                        })?;
                    pending.try_reserve(additional).map_err(|_| {
                        crate::IrError::new("SSA enum substitution work allocation failed")
                    })?;
                    pending.push(Work::Function {
                        type_parameters: &signature.type_parameters,
                        bounds: &signature.bounds,
                        witnesses: &signature.memory_witness_parameters,
                        parameter_count: signature.parameters.len(),
                    });
                    pending.push(Work::Exit(&signature.type_parameters));
                    pending.push(Work::Visit(&signature.result));
                    pending.extend(signature.parameters.iter().rev().map(Work::Visit));
                    pending.push(Work::Enter(&signature.type_parameters));
                }
                _ => {
                    completed.try_reserve(1).map_err(|_| {
                        crate::IrError::new("SSA enum substitution result allocation failed")
                    })?;
                    completed.push(ty.clone());
                }
            },
            Work::Enter(parameters) => {
                for parameter in parameters {
                    if !bound.contains_key(parameter.as_str()) {
                        bound.try_reserve(1).map_err(|_| {
                            crate::IrError::new("SSA enum substitution scope allocation failed")
                        })?;
                    }
                    *bound.entry(parameter).or_default() += 1;
                }
            }
            Work::Exit(parameters) => {
                for parameter in parameters {
                    let count = bound.get_mut(parameter.as_str()).ok_or_else(|| {
                        crate::IrError::new("SSA enum substitution scope exit is invalid")
                    })?;
                    *count -= 1;
                    if *count == 0 {
                        bound.remove(parameter.as_str());
                    }
                }
            }
            Work::Enum(id, count) => {
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    crate::IrError::new("SSA enum substitution omitted arguments")
                })?;
                let arguments = completed.split_off(split);
                completed.try_reserve(1).map_err(|_| {
                    crate::IrError::new("SSA enum substitution result allocation failed")
                })?;
                completed.push(SsaType::Enum { id, arguments });
            }
            Work::List => {
                let inner = completed.pop().ok_or_else(|| {
                    crate::IrError::new("SSA enum substitution omitted list element")
                })?;
                completed.try_reserve(1).map_err(|_| {
                    crate::IrError::new("SSA enum substitution result allocation failed")
                })?;
                completed.push(SsaType::List(Box::new(inner)));
            }
            Work::Function {
                type_parameters,
                bounds,
                witnesses,
                parameter_count,
            } => {
                let result = completed.pop().ok_or_else(|| {
                    crate::IrError::new("SSA enum substitution omitted function result")
                })?;
                let split = completed
                    .len()
                    .checked_sub(parameter_count)
                    .ok_or_else(|| {
                        crate::IrError::new("SSA enum substitution omitted function parameters")
                    })?;
                let parameters = completed.split_off(split);
                completed.try_reserve(1).map_err(|_| {
                    crate::IrError::new("SSA enum substitution result allocation failed")
                })?;
                completed.push(SsaType::Function(Box::new(crate::Signature {
                    type_parameters: type_parameters.to_vec(),
                    bounds: bounds.to_vec(),
                    memory_witness_parameters: witnesses.to_vec(),
                    parameters,
                    result: Box::new(result),
                })));
            }
        }
    }
    let result = completed
        .pop()
        .ok_or_else(|| crate::IrError::new("SSA enum substitution omitted its root"))?;
    if completed.is_empty() {
        Ok(result)
    } else {
        fail("SSA enum substitution left disconnected results")
    }
}

fn contains_enum(root: &SsaType, id: crate::EnumId) -> crate::Result<bool> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| crate::IrError::new("SSA recursive enum work allocation failed"))?;
    pending.push(root);
    while let Some(ty) = pending.pop() {
        match ty {
            SsaType::Enum {
                id: nested,
                arguments,
            } => {
                if *nested == id {
                    return Ok(true);
                }
                pending.try_reserve(arguments.len()).map_err(|_| {
                    crate::IrError::new("SSA recursive enum work allocation failed")
                })?;
                pending.extend(arguments);
            }
            SsaType::List(inner) => {
                pending.try_reserve(1).map_err(|_| {
                    crate::IrError::new("SSA recursive enum work allocation failed")
                })?;
                pending.push(inner);
            }
            _ => {}
        }
    }
    Ok(false)
}

fn contains_any_enum(root: &SsaType) -> bool {
    let mut ty = root;
    loop {
        match ty {
            SsaType::Enum { .. } => return true,
            SsaType::List(inner) => ty = inner,
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn deep_enum_metadata_type_work_is_stack_safe() {
        std::thread::Builder::new()
            .name("ssa-deep-enum-metadata-type".to_owned())
            .stack_size(128 * 1024)
            .spawn(|| {
                let depth = 1_024;
                let enumeration = crate::EnumId::new([7; 32]);
                let mut recursive = SsaType::Enum {
                    id: enumeration,
                    arguments: Vec::new(),
                };
                for _ in 0..depth {
                    recursive = SsaType::List(Box::new(recursive));
                }
                assert!(contains_enum(&recursive, enumeration).expect("recursive enum search"));
                assert!(contains_any_enum(&recursive));

                let mut parameter = SsaType::TypeParameter("t".to_owned());
                for _ in 0..depth {
                    parameter = SsaType::List(Box::new(parameter));
                }
                let substituted = substitute(&parameter, &["t".to_owned()], &[SsaType::I64])
                    .expect("deep enum substitution");
                let mut found = &substituted;
                for _ in 0..depth {
                    let SsaType::List(inner) = found else {
                        panic!("deep substitution lost list structure")
                    };
                    found = inner;
                }
                assert!(matches!(found, SsaType::I64));

                let function = SsaType::Function(Box::new(crate::Signature {
                    type_parameters: vec!["u".to_owned()],
                    bounds: Vec::new(),
                    memory_witness_parameters: Vec::new(),
                    parameters: vec![
                        SsaType::TypeParameter("t".to_owned()),
                        SsaType::TypeParameter("u".to_owned()),
                    ],
                    result: Box::new(SsaType::TypeParameter("t".to_owned())),
                }));
                let substituted = substitute(
                    &function,
                    &["t".to_owned(), "u".to_owned()],
                    &[SsaType::I64, SsaType::Bool],
                )
                .expect("function field substitution");
                let SsaType::Function(ref signature) = substituted else {
                    panic!("function substitution lost its signature")
                };
                assert_eq!(signature.parameters[0], SsaType::I64);
                assert_eq!(
                    signature.parameters[1],
                    SsaType::TypeParameter("u".to_owned())
                );
                assert_eq!(*signature.result, SsaType::I64);

                let mut nested_function = SsaType::TypeParameter("t".to_owned());
                for _ in 0..512 {
                    nested_function = SsaType::Function(Box::new(crate::Signature::monomorphic(
                        vec![nested_function],
                        SsaType::Unit,
                    )));
                }
                let nested_function =
                    substitute(&nested_function, &["t".to_owned()], &[SsaType::I64])
                        .expect("deep function field substitution");
                let mut found = &nested_function;
                for _ in 0..512 {
                    let SsaType::Function(signature) = found else {
                        panic!("deep function substitution lost signature structure")
                    };
                    found = &signature.parameters[0];
                }
                assert!(matches!(found, SsaType::I64));
            })
            .expect("spawn deep enum metadata worker")
            .join()
            .expect("deep enum metadata worker completes");
    }
}
