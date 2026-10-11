//! Postorder heights of owned composite type DAGs after substitution.
use super::*;

#[cfg(test)]
pub(super) fn validate(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    validate_inner(types, &BTreeMap::new(), work)
}

pub(super) fn validate_program(
    program: &NormalizedProgram,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    let mut nominal_children = BTreeMap::new();
    for (ty, index) in &program.record_instances {
        step(work)?;
        let layout = program.records.get(index.0 as usize).ok_or_else(missing)?;
        if !layout.owned {
            continue;
        }
        work.node::<(TypeObjectDigest, Vec<TypeObjectDigest>)>()?;
        work.reserve::<TypeObjectDigest>(layout.fields.len())?;
        let mut children = Vec::with_capacity(layout.fields.len());
        for field in layout.fields.iter() {
            step(work)?;
            children.push(field.ty);
        }
        nominal_children.insert(*ty, children);
    }
    validate_inner(&program.types, &nominal_children, work)
}

fn validate_inner(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    nominal_children: &BTreeMap<TypeObjectDigest, Vec<TypeObjectDigest>>,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    let mut heights = BTreeMap::<TypeObjectDigest, usize>::new();
    let mut active = BTreeSet::new();
    let mut pending = Vec::new();
    for (root, object) in types {
        step(work)?;
        if !matches!(
            object.form,
            TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. }
        ) && !nominal_children.contains_key(root)
        {
            continue;
        }
        work.reserve::<(TypeObjectDigest, bool)>(1)?;
        pending.push((*root, false));
        while let Some((ty, exiting)) = pending.pop() {
            step(work)?;
            if heights.contains_key(&ty) {
                continue;
            }
            let object = types.get(&ty).ok_or_else(missing)?;
            let nominal = nominal_children.get(&ty);
            work.reserve::<TypeObjectDigest>(
                object.child_type_count() + nominal.map_or(0, Vec::len),
            )?;
            let mut children = object.child_types();
            if let Some(nominal) = nominal {
                children.extend(nominal.iter().copied());
            }
            if exiting {
                let mut height = 0;
                for child in children {
                    step(work)?;
                    height = height.max(heights.get(&child).ok_or_else(missing)? + 1);
                }
                if height > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                    return Err(Diagnostic::new(
                        DiagnosticClass::Semantic,
                        "normalized_product_depth",
                        "closed owned composite exceeds the structural type depth bound",
                    ));
                }
                active.remove(&ty);
                work.node::<(TypeObjectDigest, usize)>()?;
                heights.insert(ty, height);
            } else {
                work.node::<TypeObjectDigest>()?;
                if !active.insert(ty) {
                    return Err(missing());
                }
                work.reserve::<(TypeObjectDigest, bool)>(children.len() + 1)?;
                pending.push((ty, true));
                pending.extend(children.into_iter().map(|child| (child, false)));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::{DeclarationReference, PackageId};
    use crate::platform::semantic_id::DeclarationId;

    fn insert(
        types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
        form: TypeForm,
    ) -> TypeObjectDigest {
        let object = TypeObject::new(form).unwrap();
        let ty = encode_type_object(&object).unwrap().0;
        types.insert(ty, object);
        ty
    }

    #[test]
    fn mixed_nominal_hidden_fields_and_structural_edges_share_one_depth_bound() {
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        let mut types = BTreeMap::new();
        let mut nominal = BTreeMap::new();
        let mut root = insert(&mut types, TypeForm::OwnedI64Cell);
        for depth in 0..crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            root = if depth % 2 == 0 {
                insert(&mut types, TypeForm::OwnedSequence { item: root })
            } else {
                let declaration = DeclarationReference {
                    package: PackageId::migrate(b"runtime-nominal-depth", 0),
                    declaration: DeclarationId::migrate(b"runtime-nominal-depth", depth as u64),
                };
                let ty = insert(&mut types, TypeForm::Named { declaration });
                nominal.insert(ty, vec![root]);
                ty
            };
        }
        validate_inner(&types, &nominal, &mut Budget::new(&control)).unwrap();
        insert(&mut types, TypeForm::OwnedSequence { item: root });
        let error = validate_inner(&types, &nominal, &mut Budget::new(&control)).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Semantic);
        assert_eq!(error.code, "normalized_product_depth");
    }

    #[test]
    fn nominal_layout_cycles_fail_without_recursive_host_traversal() {
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        let mut types = BTreeMap::new();
        let declaration = DeclarationReference {
            package: PackageId::migrate(b"runtime-nominal-cycle", 0),
            declaration: DeclarationId::migrate(b"runtime-nominal-cycle", 0),
        };
        let owner = insert(&mut types, TypeForm::Named { declaration });
        let sequence = insert(&mut types, TypeForm::OwnedSequence { item: owner });
        let nominal = BTreeMap::from([(owner, vec![sequence])]);
        assert!(validate_inner(&types, &nominal, &mut Budget::new(&control)).is_err());
    }
}
