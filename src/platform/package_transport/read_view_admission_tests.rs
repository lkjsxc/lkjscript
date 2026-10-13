//! Source-only controls for complete detached descriptor admission. Execution is intentionally
//! left to the selected integrated-source gate; these controls do not attest source provenance.

use super::*;
use crate::platform::kernel::*;
use crate::platform::package_interface::{
    PACKAGE_INTERFACE_CONTRACT_VERSION, PackageInterfaceOwner,
};
use crate::platform::semantic_id::{
    DeclarationId, FieldId, ParameterId, ReadScopeParameterId, RepositoryId,
};

struct Fixture {
    revision: PackageRevision,
    interface: PackageInterfaceValidation,
    function: DeclarationId,
    view: DeclarationId,
    first: ParameterId,
    second: ParameterId,
    view_type: TypeObjectDigest,
}

impl Fixture {
    fn new() -> Self {
        let seed = b"read-view-transport-inventory";
        let package = PackageId::migrate(seed, 0);
        let function = DeclarationId::migrate(seed, 0);
        let view = DeclarationId::migrate(seed, 1);
        let first = ParameterId::migrate(seed, 0);
        let second = ParameterId::migrate(seed, 1);
        let formal = ReadScopeParameter {
            id: ReadScopeParameterId::migrate(seed, 0),
            name: Name::new("source_scope").unwrap(),
        };
        let unused = ReadScopeParameter {
            id: ReadScopeParameterId::migrate(seed, 1),
            name: Name::new("unused_scope").unwrap(),
        };
        let source_scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(DeclarationReference {
                package,
                declaration: function,
            }),
            parameter: formal.id,
        });
        let owned_object = TypeObject::new(TypeForm::OwnedI64Cell).unwrap();
        let owned = encode_type_object(&owned_object).unwrap().0;
        let view_object = TypeObject::new(TypeForm::ReadViewApplied {
            declaration: DeclarationReference {
                package,
                declaration: view,
            },
            arguments: Vec::new(),
            read_scope: source_scope,
        })
        .unwrap();
        let view_type = encode_type_object(&view_object).unwrap().0;
        let record = |owner: OwnerKey, record| {
            (
                owner,
                PackageInterfaceOwner {
                    contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record,
                },
            )
        };
        let function_owner = OwnerKey::Declaration(function);
        let view_owner = OwnerKey::Declaration(view);
        let mut owners = BTreeMap::new();
        let (_, function_record) = record(
            function_owner,
            PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                header: OwnerHeader {
                    contract_version: contract::GRAPH_CONTRACT_VERSION,
                    owner: function_owner,
                    kind: OwnerKind::PureFunction,
                },
                name: Name::new("range").unwrap(),
                payload: PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                    implementation_parameters: Vec::new(),
                    requirement_parameters: Vec::new(),
                    effect_parameters: Vec::new(),
                    type_parameters: Vec::new(),
                    parameters: vec![first, second],
                    result: CallableResult {
                        ty: view_type,
                        relation: ResultRelation::ViewFrom {
                            source: first,
                            scope: source_scope,
                        },
                    },
                    read_scope_parameters: vec![formal, unused],
                    read_scope_sources: vec![ReadScopeSource {
                        source: first,
                        scope: source_scope,
                    }],
                    effect: FunctionEffect::Pure,
                }),
            }),
        );
        owners.insert(function_owner, function_record);
        owners.insert(
            view_owner,
            PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                    header: OwnerHeader {
                        contract_version: contract::GRAPH_CONTRACT_VERSION,
                        owner: view_owner,
                        kind: OwnerKind::ReadView,
                    },
                    name: Name::new("Span").unwrap(),
                    payload: PackageInterfaceDeclarationPayload::ReadView(ReadViewDeclaration {
                        type_parameters: Vec::new(),
                        scope_parameter: ReadScopeParameter {
                            id: ReadScopeParameterId::migrate(seed, 2),
                            name: Name::new("span_scope").unwrap(),
                        },
                        fields: Vec::new(),
                    }),
                }),
            },
        );
        for (id, name) in [(first, "first"), (second, "second")] {
            let owner = OwnerKey::Parameter(id);
            owners.insert(
                owner,
                PackageInterfaceOwner {
                    contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record: PackageInterfaceRecord::Parameter(ParameterRecord {
                        header: OwnerHeader {
                            contract_version: contract::GRAPH_CONTRACT_VERSION,
                            owner,
                            kind: OwnerKind::Parameter,
                        },
                        parent: ParameterParent::Function(function),
                        name: Name::new(name).unwrap(),
                        ty: owned,
                        use_mode: ParameterUse::Borrow,
                        resource_requirement: None,
                    }),
                },
            );
        }
        Self {
            revision: PackageRevision {
                contract_version: PACKAGE_REVISION_CONTRACT_VERSION,
                graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
                package,
                revision: RevisionCore {
                    contract_version:
                        crate::platform::publication::contract::REVISION_CONTRACT_VERSION,
                    graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
                    repository_id: RepositoryId::migrate(seed, 0),
                    parents: Vec::new(),
                    semantic_state: SemanticStateDigest::from_bytes([3; 32]),
                },
                interface: PackageInterfaceDigest::from_bytes([4; 32]),
                dependencies: Vec::new(),
            },
            interface: PackageInterfaceValidation {
                owners,
                type_objects: BTreeMap::from([(owned, owned_object), (view_type, view_object)]),
                reachable_objects: BTreeSet::new(),
                map_work: MapWork::default(),
            },
            function,
            view,
            first,
            second,
            view_type,
        }
    }

    fn signature(&mut self) -> &mut PackageFunctionSignature {
        let PackageInterfaceRecord::Declaration(declaration) = &mut self
            .interface
            .owners
            .get_mut(&OwnerKey::Declaration(self.function))
            .unwrap()
            .record
        else {
            unreachable!()
        };
        let PackageInterfaceDeclarationPayload::Function(function) = &mut declaration.payload
        else {
            unreachable!()
        };
        function
    }

    fn check(&self) -> Result<(), Diagnostic> {
        let digest = PackageRevisionDigest::from_bytes([5; 32]);
        validate_package_interface_closure(
            &BTreeMap::from([(digest, self.revision.clone())]),
            &BTreeMap::from([(digest, self.interface.clone())]),
        )
    }
}

#[test]
fn zero_field_view_retains_exact_source_relation_and_unused_formals() {
    let mut fixture = Fixture::new();
    fixture.check().unwrap();
    let second = fixture.second;
    let signature = fixture.signature();
    let ResultRelation::ViewFrom { scope, .. } = signature.result.relation else {
        unreachable!()
    };
    signature.result.relation = ResultRelation::ViewFrom {
        source: second,
        scope,
    };
    assert!(
        fixture.check().is_err(),
        "equal source types/scopes must not invent forwarding authority"
    );
    fixture
        .signature()
        .read_scope_sources
        .push(ReadScopeSource {
            source: second,
            scope,
        });
    fixture.check().unwrap();
    let relationship = fixture.signature().read_scope_sources[0].clone();
    fixture.signature().read_scope_sources.push(relationship);
    assert!(
        fixture.check().is_err(),
        "duplicate exact source/scope inventory cannot disappear"
    );
}

#[test]
fn complete_views_refuse_copy_private_nominal_and_hidden_metadata_occurrences() {
    let mut consuming = Fixture::new();
    let first = consuming.first;
    let PackageInterfaceRecord::Parameter(parameter) = &mut consuming
        .interface
        .owners
        .get_mut(&OwnerKey::Parameter(first))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    parameter.use_mode = ParameterUse::Consume;
    assert!(consuming.check().is_err());

    let mut private = Fixture::new();
    let declaration = private
        .interface
        .owners
        .remove(&OwnerKey::Declaration(private.view))
        .unwrap();
    let PackageInterfaceRecord::Declaration(declaration) = declaration.record else {
        unreachable!()
    };
    private.interface.owners.insert(
        OwnerKey::Declaration(private.view),
        PackageInterfaceOwner {
            contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            record: PackageInterfaceRecord::RepresentationDeclaration(declaration),
        },
    );
    assert!(
        private.check().is_err(),
        "representation evidence cannot expose a direct public view signature"
    );

    let mut hidden = Fixture::new();
    let object = TypeObject::new(TypeForm::OwnedSequence {
        item: hidden.view_type,
    })
    .unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    hidden.interface.type_objects.insert(digest, object);
    hidden.signature().result = CallableResult {
        ty: digest,
        relation: ResultRelation::Value,
    };
    assert!(
        hidden.check().is_err(),
        "an owning container cannot erase the nested affine view occurrence"
    );

    let mut unused_field = Fixture::new();
    let secret = TypeObject::new(TypeForm::Secret).unwrap();
    let digest = encode_type_object(&secret).unwrap().0;
    unused_field.interface.type_objects.insert(digest, secret);
    let PackageInterfaceRecord::Declaration(declaration) = &mut unused_field
        .interface
        .owners
        .get_mut(&OwnerKey::Declaration(unused_field.view))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::ReadView(view) = &mut declaration.payload else {
        unreachable!()
    };
    view.fields.push(ReadViewField {
        id: FieldId::migrate(b"read-view-transport-inventory", 0),
        name: Name::new("unused_secret").unwrap(),
        ty: digest,
        kind: ReadViewFieldKind::Ordinary,
    });
    assert!(
        unused_field.check().is_err(),
        "unused metadata must prove its complete ordinary Transferable closure"
    );
}

#[test]
fn view_result_mode_and_owner_profile_cannot_be_erased() {
    let mut owning = Fixture::new();
    owning.signature().result.relation = ResultRelation::Value;
    assert!(owning.check().is_err());
    let mut wrong_kind = Fixture::new();
    let source = wrong_kind.first;
    wrong_kind.signature().result.relation = ResultRelation::ReadFrom { source };
    assert!(
        wrong_kind.check().is_err(),
        "ReadFrom does not manufacture an Owned owner from a view"
    );
    let mut historical = Fixture::new();
    let function = historical
        .interface
        .owners
        .get_mut(&OwnerKey::Declaration(historical.function))
        .unwrap();
    function.contract_version = 21;
    let PackageInterfaceRecord::Declaration(declaration) = &mut function.record else {
        unreachable!()
    };
    declaration.header.contract_version = 33;
    assert!(
        historical.check().is_err(),
        "frozen paired owners cannot consume new view scopes"
    );
}
