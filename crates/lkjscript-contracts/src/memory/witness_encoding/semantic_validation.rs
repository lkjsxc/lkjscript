use super::{SemanticContractError as E, *};
use std::collections::{BTreeSet, VecDeque};

pub fn validate_semantic_descriptor(value: &SemanticDescriptor) -> Result<(), E> {
    if value.declarations.len() > MAX_SEMANTIC_DECLARATIONS {
        return Err(E("semantic declaration limit exceeded"));
    }
    let mut identities = BTreeSet::new();
    let mut prior = None;
    for declaration in &value.declarations {
        let id = declaration.identity();
        if id == [0; 32] || prior.is_some_and(|old| old >= id) || !identities.insert(id) {
            return Err(E(
                "semantic declarations must have sorted unique nonzero identities",
            ));
        }
        prior = Some(id);
        validate_declaration(declaration)?;
    }
    let mut reachable = BTreeSet::new();
    let mut queue = VecDeque::new();
    let mut work = 0;
    collect_refs(&value.root, &mut queue, &mut work)?;
    while let Some(id) = queue.pop_front() {
        if !reachable.insert(id) {
            continue;
        }
        let declaration = value
            .declarations
            .binary_search_by_key(&id, SemanticDeclaration::identity)
            .ok()
            .and_then(|index| value.declarations.get(index))
            .ok_or(E("semantic descriptor has a missing reachable declaration"))?;
        match declaration {
            SemanticDeclaration::Product(item) => {
                for field in &item.fields {
                    collect_refs(&field.ty, &mut queue, &mut work)?;
                }
            }
            SemanticDeclaration::Enum(item) => {
                for variant in &item.variants {
                    for field in &variant.fields {
                        collect_refs(&field.ty, &mut queue, &mut work)?;
                    }
                }
            }
        }
    }
    if reachable != identities {
        return Err(E("semantic descriptor contains an unreachable declaration"));
    }
    Ok(())
}

fn validate_declaration(value: &SemanticDeclaration) -> Result<(), E> {
    match value {
        SemanticDeclaration::Product(item) => {
            let mut fields = BTreeSet::new();
            for (index, field) in item.fields.iter().enumerate() {
                if field.identity == [0; 32]
                    || !fields.insert(field.identity)
                    || usize::from(field.source_order) != index
                {
                    return Err(E(
                        "product semantic fields require unique identities and exact source order",
                    ));
                }
            }
        }
        SemanticDeclaration::Enum(item) => {
            let mut variants = BTreeSet::new();
            for (vi, variant) in item.variants.iter().enumerate() {
                if variant.identity == [0; 32]
                    || !variants.insert(variant.identity)
                    || usize::from(variant.source_order) != vi
                {
                    return Err(E(
                        "enum variants require unique identities and exact source order",
                    ));
                }
                let mut fields = BTreeSet::new();
                for (fi, field) in variant.fields.iter().enumerate() {
                    if field.identity == [0; 32]
                        || !fields.insert(field.identity)
                        || usize::from(field.source_order) != fi
                    {
                        return Err(E(
                            "enum fields require unique identities and exact source order",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn missing_reachable_and_unrelated_declarations_reject() {
        let missing = SemanticDescriptor {
            root: SemanticType::Product([1; 32]),
            declarations: Vec::new(),
        };
        assert!(validate_semantic_descriptor(&missing).is_err());
        let unrelated = SemanticDescriptor {
            root: SemanticType::Primitive(SemanticPrimitiveKind::Unit),
            declarations: vec![SemanticDeclaration::Product(SemanticProductDeclaration {
                identity: [9; 32],
                fields: Vec::new(),
            })],
        };
        assert!(validate_semantic_descriptor(&unrelated).is_err());
    }
}

fn collect_refs(
    ty: &SemanticType,
    out: &mut VecDeque<[u8; 32]>,
    work: &mut usize,
) -> Result<(), E> {
    *work = work
        .checked_add(1)
        .ok_or(E("semantic type work overflow"))?;
    if *work > MAX_SEMANTIC_TYPE_NODES {
        return Err(E("semantic type node limit exceeded"));
    }
    match ty {
        SemanticType::Product(id) => out.push_back(*id),
        SemanticType::Enum {
            identity,
            arguments,
        } => {
            out.push_back(*identity);
            for ty in arguments {
                collect_refs(ty, out, work)?;
            }
        }
        SemanticType::List(ty) => collect_refs(ty, out, work)?,
        SemanticType::Function { parameters, result } => {
            for ty in parameters {
                collect_refs(ty, out, work)?;
            }
            collect_refs(result, out, work)?;
        }
        SemanticType::ForAll { body, .. } => collect_refs(body, out, work)?,
        _ => {}
    }
    Ok(())
}
