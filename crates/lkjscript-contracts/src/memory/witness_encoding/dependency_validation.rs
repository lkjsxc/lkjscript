use super::*;
use std::collections::BTreeSet;

pub fn semantic_dependency_requirements(
    descriptor: &SemanticDescriptor,
) -> Result<Vec<(ExecutableMemoryWitnessRole, SemanticType)>, SemanticContractError> {
    validate_semantic_descriptor(descriptor)?;
    let mut output = Vec::new();
    match &descriptor.root {
        SemanticType::List(element) => output.push((
            ExecutableMemoryWitnessRole::ListElement,
            (**element).clone(),
        )),
        SemanticType::Product(identity) => {
            let SemanticDeclaration::Product(item) = declaration(descriptor, *identity)? else {
                return Err(SemanticContractError(
                    "product reference resolves to non-product declaration",
                ));
            };
            for field in &item.fields {
                output.push((
                    ExecutableMemoryWitnessRole::ProductField {
                        product: *identity,
                        field: field.identity,
                        source_order: field.source_order,
                    },
                    field.ty.clone(),
                ));
            }
        }
        SemanticType::Enum {
            identity,
            arguments,
        } => {
            let SemanticDeclaration::Enum(item) = declaration(descriptor, *identity)? else {
                return Err(SemanticContractError(
                    "enum reference resolves to non-enum declaration",
                ));
            };
            if arguments.len() != item.type_parameters.len() {
                return Err(SemanticContractError(
                    "enum semantic argument arity mismatch",
                ));
            }
            for (index, argument) in arguments.iter().enumerate() {
                output.push((
                    ExecutableMemoryWitnessRole::TypeArgument {
                        constructor: *identity,
                        index: u16::try_from(index)
                            .map_err(|_| SemanticContractError("type argument index overflow"))?,
                    },
                    argument.clone(),
                ));
            }
            for variant in &item.variants {
                for field in &variant.fields {
                    output.push((
                        ExecutableMemoryWitnessRole::EnumVariantField {
                            enumeration: *identity,
                            variant: variant.identity,
                            field: field.identity,
                            variant_source_order: variant.source_order,
                            field_source_order: field.source_order,
                        },
                        substitute(&field.ty, &item.type_parameters, arguments)?,
                    ));
                }
            }
        }
        _ => {}
    }
    if output.len() > MAX_SEMANTIC_EDGES {
        return Err(SemanticContractError(
            "semantic dependency edge limit exceeded",
        ));
    }
    Ok(output)
}

pub fn validate_executable_dependencies(
    descriptor: &SemanticDescriptor,
    dependencies: &[ExecutableMemoryWitnessDependency],
) -> Result<(), SemanticContractError> {
    let expected = semantic_dependency_requirements(descriptor)?;
    if expected.len() != dependencies.len() {
        return Err(SemanticContractError(
            "executable dependency role closure is incomplete",
        ));
    }
    let mut roles = BTreeSet::new();
    for ((role, expected_ty), actual) in expected.iter().zip(dependencies) {
        if role != &actual.role || !roles.insert(actual.role.clone()) {
            return Err(SemanticContractError(
                "executable dependency roles are missing, swapped, or duplicated",
            ));
        }
        let _ = expected_ty;
    }
    Ok(())
}

pub fn direct_nominal(value: &SemanticType) -> Option<[u8; 32]> {
    match value {
        SemanticType::Product(identity) | SemanticType::Enum { identity, .. } => Some(*identity),
        _ => None,
    }
}

fn declaration(
    value: &SemanticDescriptor,
    identity: [u8; 32],
) -> Result<&SemanticDeclaration, SemanticContractError> {
    value
        .declarations
        .binary_search_by_key(&identity, SemanticDeclaration::identity)
        .ok()
        .and_then(|index| value.declarations.get(index))
        .ok_or(SemanticContractError(
            "semantic declaration reference is missing",
        ))
}

fn substitute(
    value: &SemanticType,
    parameters: &[String],
    arguments: &[SemanticType],
) -> Result<SemanticType, SemanticContractError> {
    Ok(match value {
        SemanticType::Parameter(name) => parameters
            .iter()
            .position(|item| item == name)
            .and_then(|index| arguments.get(index))
            .cloned()
            .ok_or(SemanticContractError(
                "enum field has an unbound semantic parameter",
            ))?,
        SemanticType::Enum {
            identity,
            arguments: nested,
        } => SemanticType::Enum {
            identity: *identity,
            arguments: nested
                .iter()
                .map(|item| substitute(item, parameters, arguments))
                .collect::<Result<_, _>>()?,
        },
        SemanticType::List(item) => {
            SemanticType::List(Box::new(substitute(item, parameters, arguments)?))
        }
        SemanticType::Function {
            parameters: values,
            result,
        } => SemanticType::Function {
            parameters: values
                .iter()
                .map(|item| substitute(item, parameters, arguments))
                .collect::<Result<_, _>>()?,
            result: Box::new(substitute(result, parameters, arguments)?),
        },
        SemanticType::ForAll {
            parameters: bound,
            body,
        } => SemanticType::ForAll {
            parameters: bound.clone(),
            body: Box::new(substitute(body, parameters, arguments)?),
        },
        other => other.clone(),
    })
}
