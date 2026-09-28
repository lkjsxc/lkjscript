//! Isolated foreign-signature flow corpus. Native package tests separately prove
//! complete package publication, dependency closure, linking and detached effects.
use super::*;
use crate::platform::kernel::{DependencyRecord, PackageRevisionDigest, ParameterParent};
use crate::platform::semantic_id::RevisionId;

#[test]
fn exact_package_resource_signatures_agree_with_the_disjoint_flow_oracle() {
    for nested in [false, true] {
        let (snapshot, revision, declaration, resource) = imported_helper(nested);
        assert_flow(&snapshot, true, "exact public import");
        for fault in [
            "missing-function",
            "missing-parameter",
            "missing-binding",
            "foreign-binding",
            "ordinary-use",
            "wrong-parent",
            "resource-result",
            "pure",
            "open-effect",
            "external",
        ] {
            let mut broken = snapshot.clone();
            let owners = broken.dependency_interfaces.get_mut(&revision).unwrap();
            if fault == "missing-function" {
                owners.remove(&OwnerKey::Declaration(declaration));
            } else if fault == "missing-parameter" {
                owners.remove(&OwnerKey::Parameter(resource));
            } else if matches!(
                fault,
                "missing-binding" | "foreign-binding" | "ordinary-use" | "wrong-parent"
            ) {
                let Some(PackageInterfaceRecord::Parameter(parameter)) =
                    owners.get_mut(&OwnerKey::Parameter(resource))
                else {
                    panic!("parameter");
                };
                match fault {
                    "missing-binding" => parameter.resource_requirement = None,
                    "foreign-binding" => {
                        parameter.resource_requirement.as_mut().unwrap().package =
                            PackageId::migrate(b"foreign-affine-corpus", 99);
                    }
                    "ordinary-use" => parameter.use_mode = ParameterUse::Unrestricted,
                    "wrong-parent" => {
                        parameter.parent = ParameterParent::Function(DeclarationId::migrate(
                            b"foreign-affine-corpus",
                            99,
                        ))
                    }
                    _ => unreachable!(),
                }
            } else {
                let resource_type = match &owners[&OwnerKey::Parameter(resource)] {
                    PackageInterfaceRecord::Parameter(parameter) => parameter.ty,
                    _ => unreachable!(),
                };
                let Some(PackageInterfaceRecord::Declaration(record)) =
                    owners.get_mut(&OwnerKey::Declaration(declaration))
                else {
                    panic!("declaration");
                };
                let PackageInterfaceDeclarationPayload::Function(signature) = &mut record.payload
                else {
                    panic!("signature");
                };
                match fault {
                    "resource-result" => signature.result = resource_type,
                    "pure" => signature.effect = FunctionEffect::Pure,
                    "open-effect" => signature.effect_parameters.push(
                        crate::platform::semantic_id::EffectParameterId::migrate(
                            b"foreign-affine-corpus",
                            98,
                        ),
                    ),
                    "external" => {
                        record.payload = PackageInterfaceDeclarationPayload::External(
                            crate::platform::kernel::PackageExternalSignature {
                                type_parameters: signature.type_parameters.clone(),
                                parameters: signature.parameters.clone(),
                                result: signature.result,
                            },
                        )
                    }
                    _ => unreachable!(),
                }
            }
            assert_flow(&broken, false, fault);
        }
    }
}

fn assert_flow(snapshot: &KernelSnapshot, expected: bool, case: &str) {
    assert_eq!(production_accepts(snapshot), expected, "production: {case}");
    assert_eq!(
        Reference { snapshot }.accepts(),
        expected,
        "reference: {case}"
    );
}

fn imported_helper(
    nested: bool,
) -> (
    KernelSnapshot,
    PackageRevisionDigest,
    DeclarationId,
    ParameterId,
) {
    let mut snapshot = crate::platform::execution::normalized::tests::iteration_resource_tests::type_generic_borrowed_snapshot(nested);
    let (expression, reference) = snapshot.owners.iter().find_map(|(key, owner)| {
        let OwnerRecord::Expression(record) = owner else { return None; };
        let ExpressionOperation::Call { function, .. } = &record.operation else { return None; };
        let Some(OwnerRecord::Declaration(record)) =
            snapshot.owners.get(&OwnerKey::Declaration(function.declaration)) else { return None; };
        let DeclarationPayload::Function(signature) = &record.payload else { return None; };
        signature.parameters.iter().any(|parameter|
            matches!(snapshot.owners.get(&OwnerKey::Parameter(*parameter)),
                Some(OwnerRecord::Parameter(parameter)) if parameter.use_mode == ParameterUse::Borrow)
        ).then_some((*key, *function))
    }).expect("generic resource call");
    let declaration_key = OwnerKey::Declaration(reference.declaration);
    let mut record = snapshot.owners[&declaration_key].clone();
    let OwnerRecord::Declaration(declaration) = &mut record else {
        unreachable!();
    };
    declaration.visibility = DeclarationVisibility::Public;
    let DeclarationPayload::Function(signature) = &declaration.payload else {
        unreachable!();
    };
    let resource = *signature.parameters.last().unwrap();
    let parameters = signature
        .parameters
        .iter()
        .map(|parameter| OwnerKey::Parameter(*parameter))
        .chain(
            signature
                .type_parameters
                .iter()
                .map(|parameter| OwnerKey::TypeParameter(*parameter)),
        )
        .collect::<Vec<_>>();
    let mut owners = BTreeMap::from([(
        declaration_key,
        PackageInterfaceRecord::project_public(&record)
            .unwrap()
            .unwrap(),
    )]);
    for key in parameters {
        owners.insert(
            key,
            PackageInterfaceRecord::project_public(&snapshot.owners[&key])
                .unwrap()
                .unwrap(),
        );
    }
    let package = PackageId::migrate(b"foreign-affine-corpus", 1);
    let revision = PackageRevisionDigest::from_bytes([71; 32]);
    snapshot.dependencies.insert(
        package,
        DependencyRecord {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package,
            package_revision: revision,
            semantic_revision: RevisionId::from_digest([72; 32]),
        },
    );
    snapshot.dependency_interfaces.insert(revision, owners);
    let Some(OwnerRecord::Expression(record)) = snapshot.owners.get_mut(&expression) else {
        unreachable!();
    };
    let ExpressionOperation::Call { function, .. } = &mut record.operation else {
        unreachable!();
    };
    function.package = package;
    (snapshot, revision, reference.declaration, resource)
}
