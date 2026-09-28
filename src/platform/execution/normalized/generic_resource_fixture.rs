//! Test-only extension of the existing exact-effect resource workload.
//! Each resource helper receives and returns an ordinary T; nested helpers forward
//! that same T. The concrete entry selects Unit, leaving the independent effect
//! and cleanup oracle unchanged. Public native tests exercise non-Unit instances.
use super::*;
use crate::platform::kernel::{KernelSnapshot, TypeParameterConstraints, TypeParameterRecord};
use crate::platform::semantic_id::TypeParameterId;

pub(super) fn generalize(snapshot: &mut KernelSnapshot) {
    let seed = b"type-generic-resource-workload";
    let unit = admit_snapshot_type(snapshot, TypeForm::Unit);
    let helpers = snapshot
        .owners
        .values()
        .filter_map(|record| match record {
            OwnerRecord::Declaration(record) => match &record.payload {
                DeclarationPayload::Function(function)
                    if function.parameters.last().is_some_and(|parameter| {
                        matches!(snapshot.owners.get(&OwnerKey::Parameter(*parameter)),
                            Some(OwnerRecord::Parameter(record)) if record.resource_requirement.is_some())
                    }) => Some((record.header.owner, function.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut signatures = BTreeMap::new();
    let mut next_expression = 0_u64;
    for (ordinal, (key, mut function)) in helpers.into_iter().enumerate() {
        let OwnerKey::Declaration(declaration) = key else {
            panic!("declaration key")
        };
        let parameter = TypeParameterId::migrate(seed, ordinal as u64);
        let ty = admit_snapshot_type(snapshot, TypeForm::TypeParameter { parameter });
        snapshot.owners.insert(
            OwnerKey::TypeParameter(parameter),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration,
                name: Name::new("T").unwrap(),
                constraints: TypeParameterConstraints::None,
            }),
        );
        let old_arity = function.parameters.len();
        let value = if old_arity == 1 {
            let value = ParameterId::migrate(seed, ordinal as u64);
            snapshot.owners.insert(
                OwnerKey::Parameter(value),
                OwnerRecord::Parameter(ParameterRecord {
                    header: OwnerHeader::new(OwnerKey::Parameter(value), OwnerKind::Parameter),
                    parent: ParameterParent::Function(declaration),
                    name: Name::new("value").unwrap(),
                    ty,
                    use_mode: ParameterUse::Unrestricted,
                    resource_requirement: None,
                }),
            );
            function.parameters.insert(0, value);
            value
        } else {
            assert_eq!(old_arity, 2);
            let value = function.parameters[0];
            let OwnerRecord::Parameter(record) = snapshot
                .owners
                .get_mut(&OwnerKey::Parameter(value))
                .unwrap()
            else {
                panic!("ordinary parameter")
            };
            record.ty = ty;
            value
        };
        let returned = expression(
            snapshot,
            &mut next_expression,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(value),
            },
        );
        function.body = expression(
            snapshot,
            &mut next_expression,
            ExpressionOperation::Sequence {
                items: vec![function.body, returned],
            },
        );
        function.type_parameters = vec![parameter];
        function.result = ty;
        let OwnerRecord::Declaration(record) = snapshot.owners.get_mut(&key).unwrap() else {
            panic!("helper")
        };
        record.payload = DeclarationPayload::Function(function);
        signatures.insert(declaration, (ty, value, old_arity));
    }
    assert!(!signatures.is_empty());
    let bodies = snapshot
        .owners
        .values()
        .filter_map(|record| match record {
            OwnerRecord::Declaration(record) => match &record.payload {
                DeclarationPayload::Function(function) => {
                    Some((record.header.owner, function.body))
                }
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    for (owner, body) in bodies {
        let OwnerKey::Declaration(caller) = owner else {
            panic!("caller")
        };
        let mut pending = vec![body];
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let OwnerRecord::Expression(record) =
                snapshot.owners[&OwnerKey::Expression(id)].clone()
            else {
                panic!("expression")
            };
            pending.extend(record.children().into_iter().map(|child| child.expression));
            let ExpressionOperation::Call {
                function,
                mut type_arguments,
                effect_arguments,
                requirement_arguments,
                mut arguments,
            } = record.operation
            else {
                continue;
            };
            let Some((_, _, old_arity)) = signatures.get(&function.declaration) else {
                continue;
            };
            assert_eq!(arguments.len(), *old_arity);
            assert!(type_arguments.is_empty());
            let caller_signature = signatures.get(&caller);
            type_arguments.push(caller_signature.map_or(unit, |signature| signature.0));
            if *old_arity == 1 {
                let operation =
                    caller_signature.map_or(ExpressionOperation::Unit {}, |signature| {
                        ExpressionOperation::Local {
                            value: LocalValueReference::FunctionParameter(signature.1),
                        }
                    });
                let value = expression(snapshot, &mut next_expression, operation);
                arguments.insert(0, value);
            }
            snapshot.owners.insert(
                OwnerKey::Expression(id),
                OwnerRecord::Expression(
                    ExpressionRecord::new(
                        id,
                        ExpressionOperation::Call {
                            function,
                            type_arguments,
                            effect_arguments,
                            requirement_arguments,
                            arguments,
                        },
                    )
                    .unwrap(),
                ),
            );
        }
    }
}

fn expression(
    snapshot: &mut KernelSnapshot,
    ordinal: &mut u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    *ordinal += 1;
    let id = ExpressionId::migrate(b"type-generic-resource-workload", *ordinal);
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(id),
                OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap())
            )
            .is_none()
    );
    id
}
