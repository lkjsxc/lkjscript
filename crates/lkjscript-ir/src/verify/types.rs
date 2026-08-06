use std::collections::HashSet;

use crate::verify::*;
use crate::{IrError, Program, Signature, SsaType, TraitRole};

include!("types/ownership.rs");

pub(crate) fn is_numeric(ty: &SsaType) -> bool {
    matches!(ty, SsaType::I64 | SsaType::F64)
}

pub(crate) fn verify_witness_parameters(
    signature: &Signature,
    type_parameters: &[&str],
) -> crate::Result<()> {
    if signature.memory_witness_parameters.len() > crate::MAX_MEMORY_WITNESS_PARAMETERS {
        return fail("SSA function has too many hidden memory witness parameters");
    }
    let mut prior = None;
    for requirement in &signature.memory_witness_parameters {
        let position = type_parameters
            .iter()
            .position(|parameter| *parameter == requirement.parameter)
            .ok_or_else(|| IrError::new("SSA memory witness names an undeclared type parameter"))?;
        if prior.is_some_and(|prior| prior >= position)
            || requirement.operations.is_empty()
            || requirement
                .operations
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return fail("SSA memory witness parameters are not canonical");
        }
        prior = Some(position);
    }
    Ok(())
}

enum ScopeNames<'a> {
    External(&'a [&'a str]),
    Signature(&'a [String]),
}

struct Scope<'a> {
    parent: Option<usize>,
    names: ScopeNames<'a>,
}

fn scope_contains(scopes: &[Scope<'_>], mut scope: usize, name: &str) -> bool {
    loop {
        let frame = &scopes[scope];
        let found = match &frame.names {
            ScopeNames::External(names) => names.contains(&name),
            ScopeNames::Signature(names) => names.iter().any(|candidate| candidate == name),
        };
        if found {
            return true;
        }
        let Some(parent) = frame.parent else {
            return false;
        };
        scope = parent;
    }
}

pub(crate) fn verify_type(
    program: &Program,
    ty: &SsaType,
    type_parameters: &[&str],
) -> crate::Result<()> {
    struct Work<'a> {
        ty: &'a SsaType,
        scope: usize,
        ownership_allowed: bool,
    }

    let mut scopes = vec![Scope {
        parent: None,
        names: ScopeNames::External(type_parameters),
    }];
    let mut pending = vec![Work {
        ty,
        scope: 0,
        ownership_allowed: true,
    }];
    let mut observed_work = 0_u64;
    while let Some(item) = pending.pop() {
        observed_work = observed_work
            .checked_add(1)
            .ok_or_else(|| IrError::new("SSA type verification work overflow"))?;
        match item.ty {
            SsaType::Product(product) => {
                let _metadata = product_by_id(program, *product)?;
            }
            SsaType::Enum { id, arguments } => {
                let definition = enum_by_id(program, *id)?;
                if arguments.len() != definition.type_parameters.len() {
                    return fail("SSA enum substitution arity mismatch");
                }
                let structural = program.memory.type_for(item.ty).is_some();
                pending
                    .try_reserve(arguments.len())
                    .map_err(|_| IrError::new("SSA type verification work allocation failed"))?;
                pending.extend(arguments.iter().rev().map(|argument| Work {
                    ty: argument,
                    scope: item.scope,
                    ownership_allowed: structural,
                }));
            }
            SsaType::Bytes | SsaType::ByteVector | SsaType::ByteSlice | SsaType::ByteSliceMut => {
                if !item.ownership_allowed {
                    return fail(
                        "SSA ownership/reference type has an unsupported storage position",
                    );
                }
            }
            SsaType::StructuralDestination(type_id) => {
                if !(item.ownership_allowed
                    && program
                        .memory
                        .types
                        .get(type_id.index().unwrap_or(usize::MAX))
                        .is_some_and(|metadata| metadata.id == *type_id))
                {
                    return fail("SSA structural destination has invalid private type metadata");
                }
            }
            SsaType::List(inner) => {
                pending
                    .try_reserve(1)
                    .map_err(|_| IrError::new("SSA type verification work allocation failed"))?;
                pending.push(Work {
                    ty: inner,
                    scope: item.scope,
                    ownership_allowed: false,
                });
            }
            SsaType::Function(signature) => {
                let mut names = HashSet::new();
                if signature
                    .type_parameters
                    .iter()
                    .any(|name| name.is_empty() || !names.insert(name.as_str()))
                {
                    return fail("SSA function type has invalid type parameters");
                }
                let nested_parameters: Vec<&str> = signature
                    .type_parameters
                    .iter()
                    .map(String::as_str)
                    .collect();
                let mut bounds = HashSet::new();
                for bound in &signature.bounds {
                    if !nested_parameters.contains(&bound.parameter.as_str())
                        || !bounds.insert((bound.parameter.as_str(), bound.trait_id))
                    {
                        return fail("SSA function type has malformed trait bounds");
                    }
                    let trait_metadata = trait_by_id(program, bound.trait_id)?;
                    if matches!(trait_metadata.role, TraitRole::Clone | TraitRole::Drop) {
                        return fail("SSA function type uses an unavailable core trait bound");
                    }
                }
                verify_witness_parameters(signature, &nested_parameters)?;
                if matches!(
                    signature.result.as_ref(),
                    SsaType::ByteSlice | SsaType::ByteSliceMut
                ) {
                    return fail("SSA function type cannot return a lexical reference");
                }
                let nested_scope = scopes.len();
                scopes
                    .try_reserve(1)
                    .map_err(|_| IrError::new("SSA type scope allocation failed"))?;
                scopes.push(Scope {
                    parent: Some(item.scope),
                    names: ScopeNames::Signature(&signature.type_parameters),
                });
                let additional = signature
                    .parameters
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| IrError::new("SSA type verification work size overflow"))?;
                pending
                    .try_reserve(additional)
                    .map_err(|_| IrError::new("SSA type verification work allocation failed"))?;
                pending.push(Work {
                    ty: &signature.result,
                    scope: nested_scope,
                    ownership_allowed: item.ownership_allowed,
                });
                pending.extend(signature.parameters.iter().rev().map(|parameter| Work {
                    ty: parameter,
                    scope: nested_scope,
                    ownership_allowed: item.ownership_allowed,
                }));
            }
            SsaType::TypeParameter(name) if !scope_contains(&scopes, item.scope, name) => {
                return fail(format!("SSA has unbound type parameter {name}"));
            }
            _ => {}
        }
    }
    let _ = observed_work;
    Ok(())
}
