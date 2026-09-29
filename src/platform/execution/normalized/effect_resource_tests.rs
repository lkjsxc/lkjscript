//! Effect applications must preserve the existing exact resource-flow oracle.
use super::*;
use crate::platform::kernel::{
    EffectParameterRecord, EffectParameterReference, EffectRow, KernelSnapshot,
};
use crate::platform::semantic_id::EffectParameterId;

pub(crate) fn generalize(snapshot: &mut KernelSnapshot, nonempty: bool) {
    let package = snapshot.root.package_id;
    let helpers = snapshot
        .owners
        .iter()
        .filter_map(|(key, owner)| {
            let OwnerKey::Declaration(declaration) = key else {
                return None;
            };
            let OwnerRecord::Declaration(owner) = owner else {
                return None;
            };
            let DeclarationPayload::Function(function) = &owner.payload else {
                return None;
            };
            function
                .parameters
                .iter()
                .any(|id| {
                    matches!(
                        snapshot.owners.get(&OwnerKey::Parameter(*id)),
                        Some(OwnerRecord::Parameter(p)) if p.resource_requirement.is_some()
                    )
                })
                .then_some((*declaration, function.effect.row().requirements))
        })
        .collect::<Vec<_>>();
    let mut scopes = BTreeMap::new();
    for (index, (declaration, requirements)) in helpers.into_iter().enumerate() {
        let parameter = EffectParameterId::migrate(b"effect-resource-application", index as u64);
        let reference = EffectParameterReference { package, parameter };
        snapshot.owners.insert(
            OwnerKey::EffectParameter(parameter),
            OwnerRecord::EffectParameter(EffectParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::EffectParameter(parameter),
                    OwnerKind::EffectParameter,
                ),
                declaration,
                name: Name::new("E").unwrap(),
            }),
        );
        let OwnerRecord::Declaration(owner) = snapshot
            .owners
            .get_mut(&OwnerKey::Declaration(declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &mut owner.payload else {
            unreachable!()
        };
        assert!(function.effect_parameters.is_empty());
        function.effect_parameters.push(parameter);
        let FunctionEffect::Task {
            effect_parameters, ..
        } = &mut function.effect
        else {
            unreachable!()
        };
        effect_parameters.push(reference);
        scopes.insert(declaration, (reference, requirements));
    }
    assert!(!scopes.is_empty());
    let roots = snapshot
        .owners
        .iter()
        .filter_map(|(key, owner)| {
            let OwnerKey::Declaration(declaration) = key else {
                return None;
            };
            let OwnerRecord::Declaration(owner) = owner else {
                return None;
            };
            let DeclarationPayload::Function(function) = &owner.payload else {
                return None;
            };
            Some((*declaration, function.body))
        })
        .collect::<Vec<_>>();
    for (caller, body) in roots {
        let mut pending = vec![body];
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            let OwnerRecord::Expression(expression) =
                snapshot.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
            else {
                unreachable!()
            };
            pending.extend(
                expression
                    .children()
                    .into_iter()
                    .map(|child| child.expression),
            );
            let ExpressionOperation::Call {
                function,
                effect_arguments,
                ..
            } = &mut expression.operation
            else {
                continue;
            };
            let Some((_, requirements)) = scopes.get(&function.declaration) else {
                continue;
            };
            assert!(effect_arguments.is_empty());
            let row = match scopes.get(&caller) {
                Some((reference, _)) => EffectRow {
                    requirements: vec![],
                    parameters: vec![*reference],
                },
                None => EffectRow {
                    requirements: if nonempty {
                        requirements.clone()
                    } else {
                        vec![]
                    },
                    parameters: vec![],
                },
            };
            effect_arguments.push(row);
        }
    }
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
}

pub(crate) fn snapshot(nested: bool, nonempty: bool) -> KernelSnapshot {
    let mut snapshot = super::type_generic_borrowed_snapshot(nested);
    generalize(&mut snapshot, nonempty);
    snapshot
}

#[test]
fn effect_generic_resource_applications_preserve_values_authority_and_joined_cleanup() {
    for nonempty in [false, true] {
        super::check_scoped_borrow_with_effects(true, Some(nonempty));
    }
}
