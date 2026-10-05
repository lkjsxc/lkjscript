//! Phantom family parameters remain editable through complete ordinary candidates.
use super::*;
use crate::platform::change::AuthoredImplementationParameter;
use crate::platform::kernel::{
    DeclarationReference, FunctionDeclaration, ImplementationParameter, KernelSnapshot,
    OwnedContract, OwnedImplementation, OwnedMethod, OwnedMethodImplementation,
    TypeParameterConstraints, TypeParameterRecord,
};
use crate::platform::semantic_id::{
    ExpressionId, ImplementationParameterId, MethodId, ModuleId, TypeParameterId,
};

#[derive(Clone, Copy)]
struct Ids {
    contract: DeclarationId,
    implementation: DeclarationId,
    method_function: DeclarationId,
    caller: DeclarationId,
    self_parameter: TypeParameterId,
    item_parameter: TypeParameterId,
    caller_self_parameter: TypeParameterId,
    witness_parameter: ImplementationParameterId,
    method: MethodId,
}

fn phantom_fixture() -> (KernelSnapshot, Ids) {
    let seed = b"owned-contract-phantom-parameter-deletion";
    let mut snapshot = empty_snapshot(seed);
    let package = snapshot.root.package_id;
    let module = ModuleId::migrate(seed, 0);
    let ids = Ids {
        contract: DeclarationId::migrate(seed, 0),
        implementation: DeclarationId::migrate(seed, 1),
        method_function: DeclarationId::migrate(seed, 2),
        caller: DeclarationId::migrate(seed, 3),
        self_parameter: TypeParameterId::migrate(seed, 0),
        item_parameter: TypeParameterId::migrate(seed, 1),
        caller_self_parameter: TypeParameterId::migrate(seed, 2),
        witness_parameter: ImplementationParameterId::migrate(seed, 0),
        method: MethodId::migrate(seed, 0),
    };
    let reference = |declaration| DeclarationReference {
        package,
        declaration,
    };
    let mut type_digest = |form| {
        let object = TypeObject::new(form).unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        snapshot.types.insert(digest, object);
        digest
    };
    let unit = type_digest(TypeForm::Unit);
    let cell = type_digest(TypeForm::OwnedI64Cell);
    let storage = type_digest(TypeForm::TypeParameter {
        parameter: ids.caller_self_parameter,
    });
    snapshot.owners.insert(
        OwnerKey::Module(module),
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: Name::new("phantom-family").unwrap(),
        }),
    );
    for (parameter, declaration, name) in [
        (ids.self_parameter, ids.contract, "Self"),
        (ids.item_parameter, ids.contract, "Item"),
        (ids.caller_self_parameter, ids.caller, "Storage"),
    ] {
        snapshot.owners.insert(
            OwnerKey::TypeParameter(parameter),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration,
                name: Name::new(name).unwrap(),
                constraints: TypeParameterConstraints::Owned,
            }),
        );
    }
    let mut function = |declaration, body_ordinal, type_parameters, implementation_parameters| {
        let body = ExpressionId::migrate(seed, body_ordinal);
        snapshot.owners.insert(
            OwnerKey::Expression(body),
            OwnerRecord::Expression(
                crate::platform::kernel::ExpressionRecord::new(body, ExpressionOperation::Unit {})
                    .unwrap(),
            ),
        );
        (
            declaration,
            DeclarationPayload::Function(FunctionDeclaration {
                result_borrow: None,
                implementation_parameters,
                requirement_parameters: vec![],
                effect_parameters: vec![],
                type_parameters,
                parameters: vec![],
                result: unit,
                effect: FunctionEffect::Pure,
                body,
            }),
        )
    };
    let method_function = function(ids.method_function, 0, vec![], vec![]);
    let caller = function(
        ids.caller,
        1,
        vec![ids.caller_self_parameter],
        vec![ImplementationParameter {
            id: ids.witness_parameter,
            name: Name::new("Marker").unwrap(),
            contract: reference(ids.contract),
            self_type: storage,
            type_arguments: vec![cell],
        }],
    );
    for (declaration, name, kind, payload) in [
        (
            ids.contract,
            "Marker",
            OwnerKind::OwnedContract,
            DeclarationPayload::OwnedContract(OwnedContract {
                self_parameter: ids.self_parameter,
                type_parameters: vec![ids.item_parameter],
                methods: vec![OwnedMethod {
                    result_borrow: None,
                    id: ids.method,
                    name: Name::new("inspect").unwrap(),
                    parameters: vec![],
                    result: unit,
                    effect: FunctionEffect::Pure,
                }],
            }),
        ),
        (
            ids.implementation,
            "CellMarker",
            OwnerKind::OwnedImplementation,
            DeclarationPayload::OwnedImplementation(OwnedImplementation {
                contract: reference(ids.contract),
                self_type: cell,
                type_arguments: vec![cell],
                methods: vec![OwnedMethodImplementation {
                    method: ids.method,
                    function: reference(ids.method_function),
                }],
            }),
        ),
        (
            method_function.0,
            "inspect",
            OwnerKind::PureFunction,
            method_function.1,
        ),
        (caller.0, "caller", OwnerKind::PureFunction, caller.1),
    ] {
        snapshot.owners.insert(
            OwnerKey::Declaration(declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(OwnerKey::Declaration(declaration), kind),
                module,
                name: Name::new(name).unwrap(),
                visibility: DeclarationVisibility::Private,
                payload,
            }),
        );
    }
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    (snapshot, ids)
}

#[test]
fn phantom_owned_parameter_deletion_requires_complete_application_repair_and_retains_self() {
    let parent = tempfile::tempdir().unwrap();
    let (snapshot, ids) = phantom_fixture();
    let created = GraphRepository::create(&parent.path().join("meaning"), &snapshot, None).unwrap();
    let base = created.current.head.revision;
    let delete = |parameter| AuthoredChange::DeleteOwner {
        owner: OwnerSelector::Exact {
            owner: OwnerKey::TypeParameter(parameter),
        },
        policy: AuthoredDeletePolicy::Reject,
    };
    let request = |changes| AuthoredChangeSet {
        base,
        preconditions: vec![],
        budget: ChangeBudget::default(),
        changes,
    };
    let implementation_repair = AuthoredChange::SetOwnedImplementation {
        declaration: DeclarationSelector::Id {
            declaration: ids.implementation,
        },
        contract: AuthoredDeclarationReference::Local {
            declaration: DeclarationSelector::Id {
                declaration: ids.contract,
            },
        },
        self_type: AuthoredType::OwnedI64Cell {},
        type_arguments: vec![],
        methods: vec![(
            ids.method,
            AuthoredDeclarationReference::Local {
                declaration: DeclarationSelector::Id {
                    declaration: ids.method_function,
                },
            },
        )],
    };
    let parameter_repair = AuthoredChange::SetImplementationParameters {
        declaration: DeclarationSelector::Id {
            declaration: ids.caller,
        },
        parameters: vec![AuthoredImplementationParameter {
            id: ids.witness_parameter,
            name: Name::new("Marker").unwrap(),
            contract: AuthoredDeclarationReference::Local {
                declaration: DeclarationSelector::Id {
                    declaration: ids.contract,
                },
            },
            self_type: AuthoredType::TypeParameter {
                parameter: AuthoredTypeParameterReference::Id {
                    parameter: ids.caller_self_parameter,
                },
            },
            type_arguments: vec![],
        }],
    };
    for repairs in [
        vec![],
        vec![implementation_repair.clone()],
        vec![parameter_repair.clone()],
    ] {
        let mut changes = repairs;
        changes.push(delete(ids.item_parameter));
        let errors = created
            .repository
            .prepare_authored_change(&request(changes), PublicationOptions::default())
            .expect_err("every phantom application must match the changed family arity");
        assert!(
            errors
                .iter()
                .any(|error| error.code == "kernel_owned_contract"
                    && error.message.contains("arity")),
            "{errors:?}"
        );
        assert_eq!(created.repository.current().unwrap().head.revision, base);
    }
    let errors = created
        .repository
        .prepare_authored_change(
            &request(vec![delete(ids.self_parameter)]),
            PublicationOptions::default(),
        )
        .expect_err("distinguished Self cannot detach from a retained contract");
    assert_eq!(errors[0].code, "change_delete_owned_contract_self");
    assert_eq!(created.repository.current().unwrap().head.revision, base);

    let prepared = created
        .repository
        .prepare_authored_change(
            &request(vec![
                implementation_repair,
                parameter_repair,
                delete(ids.item_parameter),
            ]),
            PublicationOptions::default(),
        )
        .expect("complete applications permit deletion of the unused family parameter");
    created.repository.publish(&prepared.publication).unwrap();
    let current = created.repository.view_current().unwrap();
    let accepted = current.reconstruct_full_oracle().unwrap().value;
    crate::platform::kernel::validate_full(&accepted).unwrap();
    assert!(
        !accepted
            .owners
            .contains_key(&OwnerKey::TypeParameter(ids.item_parameter))
    );
    assert!(
        accepted
            .owners
            .contains_key(&OwnerKey::TypeParameter(ids.self_parameter))
    );
    assert!(
        accepted
            .retirements
            .contains_key(&OwnerKey::TypeParameter(ids.item_parameter))
    );
    assert_eq!(accepted.owners.len(), snapshot.owners.len() - 1);
    let OwnerRecord::Declaration(contract) = &accepted.owners[&OwnerKey::Declaration(ids.contract)]
    else {
        panic!("retained contract");
    };
    let DeclarationPayload::OwnedContract(contract) = &contract.payload else {
        panic!("retained contract kind");
    };
    assert_eq!(contract.self_parameter, ids.self_parameter);
    assert!(contract.type_parameters.is_empty());
}
