use super::*;
use crate::platform::persistent_map::MemoryPageStore;

fn manifest(version: u16) -> ArtifactManifest {
    let package = PackageId::migrate(b"read-view-artifact-wire", 0);
    let empty = PersistentMap::empty(&mut MemoryPageStore::default(), &mut MapWork::default())
        .unwrap()
        .root();
    let (graph, compiler, bytecode) = (34, 34, 28);
    ArtifactManifest {
        contract_version: version,
        graph_contract_version: graph,
        compiler_contract_version: compiler,
        bytecode_contract_version: bytecode,
        compilation_manifest_contract_version: COMPILATION_MANIFEST_CONTRACT_VERSION,
        root_package: package,
        packages: vec![ArtifactPackage {
            repository_id: RepositoryId::migrate(b"read-view-artifact-wire", 0),
            package,
            package_revision: PackageRevisionDigest::from_bytes([1; 32]),
            semantic_revision: RevisionId::from_digest([2; 32]),
            semantic_state: SemanticStateDigest::from_bytes([3; 32]),
            interface: PackageInterfaceDigest::from_bytes([4; 32]),
            interface_owners: empty,
            compilation: CompilationManifestDigest::from_bytes([5; 32]),
            runtime_owners: vec![],
            reference_owners: empty,
        }],
        closure: ArtifactClosureDigest::from_bytes([6; 32]),
        object_count: 1,
        object_bytes: 1,
    }
}

#[test]
fn artifact_manifest_and_bundle_bind_the_current_canonical_schema() {
    let expected = manifest(41);
    let (digest, bytes) = expected.encode().unwrap();
    assert_eq!(&bytes[..8], &ARTIFACT_MANIFEST_MAGIC);
    assert_eq!(ArtifactManifest::decode(&bytes, digest).unwrap(), expected);
    let mut foreign = bytes;
    foreign[8] ^= 1;
    let digest = ArtifactManifestDigest::from_bytes(
        ObjectKey::for_bytes(ObjectDomain::ArtifactManifest, &foreign)
            .digest
            .bytes(),
    );
    assert_eq!(
        ArtifactManifest::decode(&foreign, digest).unwrap_err().code,
        "packed_schema"
    );

    let bytes = include_bytes!("../../../packages/standard/generated/standard.lkja");
    let loaded = load_artifact(bytes).unwrap();
    assert_eq!(loaded.manifest.contract_version, ARTIFACT_CONTRACT_VERSION);
    let mut foreign = bytes.to_vec();
    foreign[8] ^= 1;
    assert_eq!(
        load_artifact(&foreign).unwrap_err().code,
        "artifact_bundle_schema"
    );
    let mut truncated = bytes.to_vec();
    truncated.truncate(BUNDLE_HEADER_BYTES - 1);
    assert_eq!(
        load_artifact(&truncated).unwrap_err().code,
        "artifact_bundle_length"
    );
}

#[test]
fn read_view_runtime_inventory_binds_unused_fields_and_scope_formal_exactly() {
    use crate::platform::kernel::{
        DeclarationRecord, DeclarationVisibility, OwnerHeader, ReadScopeBinder, ReadScopeReference,
        ReadViewDeclaration, ReadViewField, ReadViewFieldKind,
    };
    use crate::platform::semantic_id::{FieldId, ModuleId, ReadScopeParameterId};
    let declaration = DeclarationReference {
        package: PackageId::migrate(b"read-view-artifact-inventory", 0),
        declaration: DeclarationId::migrate(b"read-view-artifact-inventory", 0),
    };
    let scope_parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(b"read-view-artifact-inventory", 0),
        name: Name::new("source_scope").unwrap(),
    };
    let scope = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration),
        parameter: scope_parameter.id,
    });
    let view = ReadViewDeclaration {
        type_parameters: vec![],
        scope_parameter,
        fields: vec![
            ReadViewField {
                id: FieldId::migrate(b"read-view-artifact-inventory", 0),
                name: Name::new("metadata").unwrap(),
                ty: TypeObjectDigest::from_bytes([1; 32]),
                kind: ReadViewFieldKind::Ordinary,
            },
            ReadViewField {
                id: FieldId::migrate(b"read-view-artifact-inventory", 1),
                name: Name::new("unused_reference").unwrap(),
                ty: TypeObjectDigest::from_bytes([2; 32]),
                kind: ReadViewFieldKind::ReadReference { read_scope: scope },
            },
        ],
    };
    let expectation = RuntimeOwnerExpectation::ReadView(view.clone());
    let record = |view| {
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(declaration.declaration),
                OwnerKind::ReadView,
            ),
            module: ModuleId::migrate(b"read-view-artifact-inventory", 0),
            name: Name::new("SelectedView").unwrap(),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::ReadView(view),
        })
    };
    assert!(expectation.matches(&record(view.clone())));
    let mut missing = view.clone();
    missing.fields.pop();
    assert!(!expectation.matches(&record(missing)));
    let mut reordered = view.clone();
    reordered.fields.reverse();
    assert!(!expectation.matches(&record(reordered)));
    let mut retagged = view.clone();
    retagged.fields[1].kind = ReadViewFieldKind::Ordinary;
    assert!(!expectation.matches(&record(retagged)));
    let mut foreign_scope = view;
    foreign_scope.scope_parameter.id =
        ReadScopeParameterId::migrate(b"read-view-artifact-inventory", 1);
    assert!(!expectation.matches(&record(foreign_scope)));
}

#[test]
fn callable_runtime_inventory_binds_view_source_and_unused_scope_relationships() {
    // These isolated metadata correspondence checks create no semantic authority.
    // Full packages still traverse occurrence, expression, affine and code admission.
    use crate::platform::kernel::{
        DeclarationRecord, DeclarationVisibility, FunctionDeclaration, OwnerHeader,
        ReadScopeBinder, ReadScopeReference,
    };
    use crate::platform::semantic_id::{ExpressionId, ModuleId, ReadScopeParameterId};
    let declaration = DeclarationReference {
        package: PackageId::migrate(b"read-view-artifact-signature", 0),
        declaration: DeclarationId::migrate(b"read-view-artifact-signature", 0),
    };
    let parameters = vec![
        ParameterId::migrate(b"read-view-artifact-signature", 0),
        ParameterId::migrate(b"read-view-artifact-signature", 1),
    ];
    let scopes = vec![
        ReadScopeParameter {
            id: ReadScopeParameterId::migrate(b"read-view-artifact-signature", 0),
            name: Name::new("selected").unwrap(),
        },
        ReadScopeParameter {
            id: ReadScopeParameterId::migrate(b"read-view-artifact-signature", 1),
            name: Name::new("unused").unwrap(),
        },
    ];
    let scope = |index: usize| {
        ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(declaration),
            parameter: scopes[index].id,
        })
    };
    let function = FunctionDeclaration {
        implementation_parameters: vec![],
        requirement_parameters: vec![],
        effect_parameters: vec![],
        type_parameters: vec![],
        parameters: parameters.clone(),
        read_scope_parameters: scopes.clone(),
        read_scope_sources: vec![
            ReadScopeSource {
                source: parameters[0],
                scope: scope(0),
            },
            ReadScopeSource {
                source: parameters[1],
                scope: scope(1),
            },
        ],
        result: CallableResult {
            ty: TypeObjectDigest::from_bytes([7; 32]),
            relation: ResultRelation::ViewFrom {
                source: parameters[0],
                scope: scope(0),
            },
        },
        effect: FunctionEffect::Pure,
        body: ExpressionId::migrate(b"read-view-artifact-signature", 0),
    };
    let expectation = RuntimeOwnerExpectation::ResourceFunction {
        implementation_parameters: vec![],
        requirement_parameters: vec![],
        effect_parameters: vec![],
        type_parameters: vec![],
        parameters,
        read_scope_parameters: function.read_scope_parameters.clone(),
        read_scope_sources: function.read_scope_sources.clone(),
        result: function.result,
        effect: function.effect.clone(),
        requirements: vec![],
    };
    let record = |function| {
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(declaration.declaration),
                OwnerKind::PureFunction,
            ),
            module: ModuleId::migrate(b"read-view-artifact-signature", 0),
            name: Name::new("select_view").unwrap(),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(function),
        })
    };
    assert!(expectation.matches(&record(function.clone())));
    let mut wrong_source = function.clone();
    if let ResultRelation::ViewFrom { source, .. } = &mut wrong_source.result.relation {
        *source = wrong_source.parameters[1];
    }
    assert!(!expectation.matches(&record(wrong_source)));
    let mut wrong_binder = function.clone();
    if let ResultRelation::ViewFrom {
        scope: ReadScopeArgument::Parameter(reference),
        ..
    } = &mut wrong_binder.result.relation
    {
        reference.binder = ReadScopeBinder::Declaration(DeclarationReference {
            declaration: DeclarationId::migrate(b"read-view-artifact-signature", 1),
            ..declaration
        });
    }
    assert!(!expectation.matches(&record(wrong_binder)));
    let mut missing_unused = function.clone();
    missing_unused.read_scope_sources.pop();
    assert!(!expectation.matches(&record(missing_unused)));
    let mut wrong_unused = function;
    wrong_unused.read_scope_sources[1].scope = scope(0);
    assert!(!expectation.matches(&record(wrong_unused)));
}
