use super::*;
use crate::platform::persistent_map::MemoryPageStore;

fn manifest(version: u16) -> ArtifactManifest {
    let package = PackageId::migrate(b"read-view-artifact-wire", 0);
    let empty = PersistentMap::empty(&mut MemoryPageStore::default(), &mut MapWork::default())
        .unwrap()
        .root();
    let (graph, compiler, bytecode) = match version {
        40 => (33, 33, 27),
        41 => (34, 34, 28),
        _ => unreachable!(),
    };
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
fn read_view_writer_and_retained_predecessor_have_distinct_exact_wire_profiles() {
    assert_eq!(ARTIFACT_CONTRACT_VERSION, 41);
    assert_eq!(
        ARTIFACT_MANIFEST_CONTRACT_IDENTITY,
        "lkjscript-artifact-manifest-41"
    );
    assert_eq!(
        ARTIFACT_BUNDLE_CONTRACT_IDENTITY,
        "lkjscript-artifact-bundle-41"
    );
    for version in [40, 41] {
        let expected = manifest(version);
        let (digest, bytes) = expected.encode().unwrap();
        assert_eq!(
            &bytes[..8],
            if version == 40 {
                b"LKJAMF40"
            } else {
                b"LKJAMF41"
            }
        );
        assert_eq!(ArtifactManifest::decode(&bytes, digest).unwrap(), expected);
        let mut relabelled = bytes;
        relabelled[..8].copy_from_slice(if version == 40 {
            b"LKJAMF41"
        } else {
            b"LKJAMF40"
        });
        let forged_digest = ArtifactManifestDigest::from_bytes(
            ObjectKey::for_bytes(ObjectDomain::ArtifactManifest, &relabelled)
                .digest
                .bytes(),
        );
        assert!(ArtifactManifest::decode(&relabelled, forged_digest).is_err());
    }
    for (version, graph, compiler, bytecode) in [
        (40, 34, 34, 28),
        (40, 33, 34, 28),
        (40, 34, 33, 27),
        (41, 33, 33, 27),
        (41, 34, 33, 27),
        (41, 33, 34, 28),
    ] {
        let mut mixed = manifest(version);
        mixed.graph_contract_version = graph;
        mixed.compiler_contract_version = compiler;
        mixed.bytecode_contract_version = bytecode;
        assert_eq!(
            mixed.encode().unwrap_err().code,
            "artifact_manifest_contract"
        );
    }
}

#[test]
fn genuine_retained_standard_preserves_its_original_profile_and_canonical_bytes() {
    use sha2::Digest;
    // This original .95 asset is an independent producer oracle. The current loader
    // must admit the original package/unit closure without upgrading its envelope.
    let bytes = include_bytes!("fixtures/standard-artifact40.lkja");
    let sha256 = sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        sha256,
        "05649e55bc2bda66375a9bc380f0446bce524bbb46cbe761be3c485131cbfa10"
    );
    assert_eq!(&bytes[..8], b"LKJART40");
    let loaded = load_artifact(bytes).unwrap();
    assert_eq!(
        (
            loaded.manifest.contract_version,
            loaded.manifest.graph_contract_version,
            loaded.manifest.compiler_contract_version,
            loaded.manifest.bytecode_contract_version,
        ),
        (40, 33, 33, 27),
    );
    let (manifest_digest, encoded) = loaded.manifest.encode().unwrap();
    let length = read_u64(bytes, 12, "test_manifest_length").unwrap() as usize;
    assert_eq!(
        &encoded,
        &bytes[BUNDLE_HEADER_BYTES..BUNDLE_HEADER_BYTES + length]
    );
    assert_eq!(
        manifest_digest.bytes(),
        read_array::<32>(bytes, 28, "test_manifest_digest").unwrap()
    );

    // The regenerated maintained asset remains an independent current-profile control.
    let current_bytes = include_bytes!("../../../packages/standard/generated/standard.lkja");
    assert_eq!(&current_bytes[..8], b"LKJART41");
    let current = load_artifact(current_bytes).unwrap();
    assert_eq!(
        (
            current.manifest.contract_version,
            current.manifest.graph_contract_version,
            current.manifest.compiler_contract_version,
            current.manifest.bytecode_contract_version
        ),
        (41, 34, 34, 28)
    );
    let (digest, encoded) = current.manifest.encode().unwrap();
    let length = read_u64(current_bytes, 12, "test_manifest_length").unwrap() as usize;
    assert_eq!(
        &encoded,
        &current_bytes[BUNDLE_HEADER_BYTES..BUNDLE_HEADER_BYTES + length]
    );
    assert_eq!(
        digest.bytes(),
        read_array::<32>(current_bytes, 28, "test_manifest_digest").unwrap()
    );
}

#[test]
fn predecessor_type_admission_finds_a_view_in_an_unused_phantom_actual() {
    use crate::platform::kernel::{ReadScopeBinder, ReadScopeReference};
    use crate::platform::semantic_id::ReadScopeParameterId;
    let declaration = DeclarationReference {
        package: PackageId::migrate(b"read-view-artifact-generation", 0),
        declaration: DeclarationId::migrate(b"read-view-artifact-generation", 0),
    };
    let view = TypeObject::new(TypeForm::ReadViewApplied {
        declaration,
        arguments: vec![],
        read_scope: ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(declaration),
            parameter: ReadScopeParameterId::migrate(b"read-view-artifact-generation", 0),
        }),
    })
    .unwrap();
    let view_digest = encode_type_object(&view).unwrap().0;
    let phantom = TypeObject::new(TypeForm::Applied {
        declaration: DeclarationReference {
            declaration: DeclarationId::migrate(b"read-view-artifact-generation", 1),
            ..declaration
        },
        arguments: vec![view_digest],
    })
    .unwrap();
    let phantom_digest = encode_type_object(&phantom).unwrap().0;
    let types = BTreeMap::from([(view_digest, view), (phantom_digest, phantom)]);
    assert_eq!(
        validate_read_view_generation_closure(BTreeSet::from([phantom_digest]), &types)
            .unwrap_err()
            .code,
        "kernel_read_view_generation",
    );
    assert!(validate_read_view_generation_closure(BTreeSet::new(), &types).is_ok());
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
