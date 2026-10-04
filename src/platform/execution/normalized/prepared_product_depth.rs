//! Postorder heights of owned composite type DAGs after substitution.
use super::*;

pub(super) fn validate(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
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
        ) {
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
            work.reserve::<TypeObjectDigest>(object.child_type_count())?;
            let children = object.child_types();
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
