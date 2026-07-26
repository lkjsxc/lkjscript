use std::collections::HashSet;

use crate::verify::*;
use crate::{IrError, Program, Signature, SsaType, TraitRole};

pub(crate) fn supports_value_equality(ty: &SsaType) -> bool {
    match ty {
        SsaType::Unit
        | SsaType::Bool
        | SsaType::I64
        | SsaType::F64
        | SsaType::Str
        | SsaType::Path
        | SsaType::Symbol => true,
        SsaType::Enum { arguments, .. } => arguments.iter().all(supports_value_equality),
        _ => false,
    }
}

pub(crate) fn signature_contains_ownership(signature: &Signature) -> bool {
    signature.parameters.iter().any(contains_ownership_type)
        || contains_ownership_type(&signature.result)
}

pub(crate) fn contains_ownership_type(ty: &SsaType) -> bool {
    match ty {
        SsaType::Owned(_) | SsaType::Ref(_) | SsaType::RefMut(_) => true,
        SsaType::List(inner) => contains_ownership_type(inner),
        SsaType::Enum { arguments, .. } => arguments.iter().any(contains_ownership_type),
        SsaType::Function(signature) => {
            signature.parameters.iter().any(contains_ownership_type)
                || contains_ownership_type(&signature.result)
        }
        _ => false,
    }
}

pub(crate) fn is_owned_buf(ty: &SsaType) -> bool {
    matches!(ty, SsaType::Owned(inner) if inner.as_ref() == &SsaType::Buf)
}

pub(crate) fn is_owned_value(ty: &SsaType) -> bool {
    is_owned_buf(ty) || ty == &SsaType::Handle
}

pub(crate) fn is_affine(ty: &SsaType) -> bool {
    is_owned_value(ty) || matches!(ty, SsaType::RefMut(inner) if inner.as_ref() == &SsaType::Buf)
}

pub(crate) fn is_numeric(ty: &SsaType) -> bool {
    matches!(ty, SsaType::I64 | SsaType::F64)
}

pub(crate) fn verify_type(
    program: &Program,
    ty: &SsaType,
    type_parameters: &[&str],
) -> crate::Result<()> {
    let mut work = 0;
    verify_type_at(program, ty, type_parameters, 0, &mut work, true)
}

pub(crate) fn verify_type_at(
    program: &Program,
    ty: &SsaType,
    type_parameters: &[&str],
    depth: usize,
    work: &mut usize,
    ownership_allowed: bool,
) -> crate::Result<()> {
    if depth > TYPE_VERIFY_MAX_DEPTH {
        return fail(format!("SSA type nesting exceeds {TYPE_VERIFY_MAX_DEPTH}"));
    }
    *work = work
        .checked_add(1)
        .ok_or_else(|| IrError::new("SSA type verification work overflow"))?;
    if *work > TYPE_VERIFY_MAX_WORK {
        return fail(format!(
            "SSA type verification work exceeds {TYPE_VERIFY_MAX_WORK}"
        ));
    }
    match ty {
        SsaType::Product(product) => {
            let _metadata = product_by_id(program, *product)?;
            Ok(())
        }
        SsaType::Enum { id, arguments } => {
            let definition = enum_by_id(program, *id)?;
            if arguments.len() != definition.type_parameters.len() {
                return fail("SSA enum substitution arity mismatch");
            }
            for argument in arguments {
                verify_type_at(program, argument, type_parameters, depth + 1, work, false)?;
            }
            Ok(())
        }
        SsaType::Owned(inner) | SsaType::Ref(inner) | SsaType::RefMut(inner) => {
            if !ownership_allowed {
                return fail("SSA ownership/reference type has an unsupported storage position");
            }
            if inner.as_ref() != &SsaType::Buf {
                return fail("SSA ownership/reference type must contain exact Buf");
            }
            Ok(())
        }
        SsaType::List(item) => {
            verify_type_at(program, item, type_parameters, depth + 1, work, false)
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
            let mut nested_scope: Vec<&str> = type_parameters
                .iter()
                .copied()
                .filter(|outer| !nested_parameters.contains(outer))
                .collect();
            nested_scope.extend(nested_parameters.iter().copied());
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
            for parameter in &signature.parameters {
                verify_type_at(
                    program,
                    parameter,
                    &nested_scope,
                    depth + 1,
                    work,
                    ownership_allowed,
                )?;
            }
            if matches!(
                signature.result.as_ref(),
                SsaType::Ref(_) | SsaType::RefMut(_)
            ) {
                return fail("SSA function type cannot return a lexical reference");
            }
            verify_type_at(
                program,
                &signature.result,
                &nested_scope,
                depth + 1,
                work,
                ownership_allowed,
            )
        }
        SsaType::TypeParameter(name) => {
            if type_parameters.contains(&name.as_str()) {
                Ok(())
            } else {
                fail(format!("SSA has unbound type parameter {name}"))
            }
        }
        _ => Ok(()),
    }
}
