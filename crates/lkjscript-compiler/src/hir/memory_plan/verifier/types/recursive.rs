use super::*;

pub(crate) fn verified_recursive_fields(
    program: &hir::Program,
    key: &VerifiedDeclarationKey,
) -> Result<Vec<(Type, MemoryTypePathElement)>> {
    match key {
        VerifiedDeclarationKey::Product(name) => {
            let item = program
                .products
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| Error::msg("memory verifier lost recursive product"))?;
            item.fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    Ok((
                        field.ty.clone(),
                        MemoryTypePathElement::ProductField {
                            index: index_u32(index)?,
                            name: field.name.clone(),
                        },
                    ))
                })
                .collect()
        }
        VerifiedDeclarationKey::Enum(id) => {
            let item = program
                .enums
                .iter()
                .find(|item| item.id.bytes() == *id)
                .ok_or_else(|| Error::msg("memory verifier lost recursive enum"))?;
            let mut fields = Vec::new();
            for (variant_index, variant) in item.variants.iter().enumerate() {
                for (field_index, field) in variant.fields.iter().enumerate() {
                    fields.push((
                        field.ty.clone(),
                        MemoryTypePathElement::EnumVariantField {
                            variant_index: index_u32(variant_index)?,
                            variant: variant.id.bytes(),
                            field_index: index_u32(field_index)?,
                            field: field.id.bytes(),
                        },
                    ));
                }
            }
            Ok(fields)
        }
    }
}

pub(crate) fn verified_recursive_substitutions(
    program: &hir::Program,
    declaration: &VerifiedDeclarationKey,
    root: &VerifiedDeclarationKey,
    arguments: &[Type],
) -> Result<HashMap<String, Type>> {
    if declaration != root {
        return Ok(HashMap::new());
    }
    let VerifiedDeclarationKey::Enum(id) = declaration else {
        return Ok(HashMap::new());
    };
    let item = program
        .enums
        .iter()
        .find(|item| item.id.bytes() == *id)
        .ok_or_else(|| Error::msg("memory verifier lost recursive substitution"))?;
    if item.type_parameters.len() != arguments.len() {
        return Err(Error::msg("memory verifier recursive enum arity mismatch"));
    }
    Ok(item
        .type_parameters
        .iter()
        .cloned()
        .zip(arguments.iter().cloned())
        .collect())
}

pub(crate) fn verified_recursive_root(
    program: &hir::Program,
    key: &VerifiedDeclarationKey,
    arguments: &[Type],
) -> Result<Type> {
    match key {
        VerifiedDeclarationKey::Product(name) => Ok(Type::Product(name.clone())),
        VerifiedDeclarationKey::Enum(id) => {
            let item = program
                .enums
                .iter()
                .find(|item| item.id.bytes() == *id)
                .ok_or_else(|| Error::msg("memory verifier lost recursive enum identity"))?;
            Ok(Type::Enum {
                id: item.id,
                name: item.name.clone(),
                arguments: arguments.to_vec(),
            })
        }
    }
}

pub(crate) fn verified_recursive_mixed(
    fact: VerifiedExpectedType,
    path: Vec<MemoryTypePathElement>,
) -> VerifiedDerived {
    VerifiedDerived {
        mode: fact.derived.mode,
        contains_borrow: fact.derived.contains_borrow,
        contains_dynamic_owner: fact.derived.contains_dynamic_owner,
        closure: MemoryClosureFact {
            class: MemoryClosureClass::IllegalMixedBridge,
            blocker_path: path,
            blocker_type: Some(fact.ty),
            blocker_reason: Some(MemoryBlockerReason::DynamicDeterministicOwner),
            mixed_direction: Some(MemoryMixedBridgeDirection::LegacyContainsDeterministic),
        },
    }
}
