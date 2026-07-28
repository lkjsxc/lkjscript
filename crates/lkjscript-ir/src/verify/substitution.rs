use std::collections::{HashMap, HashSet};

use crate::verify::*;
use crate::{Signature, SsaType};

pub(crate) fn bind_type<'a>(
    declared: &'a SsaType,
    resolved: &SsaType,
    permitted: &HashSet<&'a str>,
    substitutions: &mut HashMap<&'a str, SsaType>,
) -> crate::Result<()> {
    match (declared, resolved) {
        (SsaType::TypeParameter(name), resolved) if permitted.contains(name.as_str()) => {
            if let Some(previous) = substitutions.get(name.as_str()) {
                if previous != resolved {
                    return fail("SSA generic call has conflicting type substitutions");
                }
            } else {
                substitutions.insert(name, resolved.clone());
            }
            Ok(())
        }
        (SsaType::List(left), SsaType::List(right)) => {
            bind_type(left, right, permitted, substitutions)
        }
        (
            SsaType::Enum {
                id: left_id,
                arguments: left,
            },
            SsaType::Enum {
                id: right_id,
                arguments: right,
            },
        ) if left_id == right_id && left.len() == right.len() => {
            for (left, right) in left.iter().zip(right) {
                bind_type(left, right, permitted, substitutions)?;
            }
            Ok(())
        }
        (SsaType::Function(left), SsaType::Function(right)) => {
            if left.type_parameters != right.type_parameters
                || left.bounds != right.bounds
                || left.parameters.len() != right.parameters.len()
            {
                return fail("SSA generic function type identity or arity mismatch");
            }
            let nested_permitted: HashSet<&str> = permitted
                .iter()
                .copied()
                .filter(|name| !left.type_parameters.iter().any(|nested| nested == name))
                .collect();
            for (left, right) in left.parameters.iter().zip(&right.parameters) {
                bind_type(left, right, &nested_permitted, substitutions)?;
            }
            bind_type(
                &left.result,
                &right.result,
                &nested_permitted,
                substitutions,
            )
        }
        (left, right) if left == right => Ok(()),
        _ => fail("SSA resolved call type is incompatible with declaration"),
    }
}

pub(crate) fn substitute_type(ty: &SsaType, substitutions: &HashMap<&str, SsaType>) -> SsaType {
    match ty {
        SsaType::TypeParameter(name) => substitutions
            .get(name.as_str())
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        SsaType::List(item) => SsaType::List(Box::new(substitute_type(item, substitutions))),
        SsaType::Enum { id, arguments } => SsaType::Enum {
            id: *id,
            arguments: arguments
                .iter()
                .map(|item| substitute_type(item, substitutions))
                .collect(),
        },
        SsaType::Function(signature) => {
            let nested_substitutions: HashMap<&str, SsaType> = substitutions
                .iter()
                .filter(|(name, _)| {
                    !signature
                        .type_parameters
                        .iter()
                        .any(|nested| nested == **name)
                })
                .map(|(name, ty)| (*name, ty.clone()))
                .collect();
            SsaType::Function(Box::new(Signature {
                type_parameters: signature.type_parameters.clone(),
                bounds: signature.bounds.clone(),
                parameters: signature
                    .parameters
                    .iter()
                    .map(|ty| substitute_type(ty, &nested_substitutions))
                    .collect(),
                result: Box::new(substitute_type(&signature.result, &nested_substitutions)),
            }))
        }
        _ => ty.clone(),
    }
}
