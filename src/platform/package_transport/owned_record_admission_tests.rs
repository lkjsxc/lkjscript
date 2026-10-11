// Coherently rehash hostile immutable body content; no authoring authority is used to bypass admission.
const OWNED_RECORD_TRANSPORT_SOURCE: &str = r#"declarations.begin
(units
  (module create provider
    (owned-record create Inner (visibility private)
      (field create value (type I64)))
    (owned-record create Envelope (visibility public)
      (field create inner (type Inner)))
    (owned-record create Other (visibility private)
      (field create value (type I64)))
    (owned-record create Phantom (visibility private)
      (type-parameter create T (constraint transferable))
      (field create value (type I64)))
    (external create buffer (visibility private)
      (implementation core.buffer.empty) (returns ByteBuffer))
    (function create make (visibility public) (effect pure) (returns Envelope)
      (body (let (binding inner (type Inner)
        (pack-owned (type Inner) (field value (i64 7))))
        (in (pack-owned (type Envelope) (field inner (local inner)))))))
    (function create unused (visibility private) (effect pure)
      (parameter create value (type (Phantom I64)) (use consume))
      (returns Unit) (body (unit))))
  (module create outsider
    (function create idle (visibility private) (effect pure)
      (returns Unit) (body (unit)))))
declarations.end"#;

fn owned_record_transport() -> AdmittedClosure {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        OWNED_RECORD_TRANSPORT_SOURCE,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let repository =
        GraphRepository::create(&directory.path().join("owned-records"), &snapshot, None)
            .unwrap()
            .repository;
    repository.export_package_container().unwrap()
}

fn reject_owned_record_transport(hostile: &PackageContainer) {
    let bytes = hostile.encode().unwrap();
    let decoded = PackageContainer::decode(&bytes, hostile.root.transport).unwrap();
    let error = decoded.admit().unwrap_err();
    assert!(
        error.code.starts_with("kernel_") || error.code == "intrinsic_signature",
        "{error:?}"
    );
    assert!(crate::platform::package_transport::oracle::reconstruct(&decoded).is_err());
    assert_not_ready(&bytes, hostile.root.transport);
}

#[test]
fn owned_record_transport_reconstructs_private_representation_without_exporting_names() {
    let original = owned_record_transport();
    let independently =
        crate::platform::package_transport::oracle::reconstruct(&original.container).unwrap();
    let package = &original.packages[&original.container.root.package_revision];
    let inner = package
        .snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Declaration(record) if record.name.as_str() == "Inner" => {
                Some(record.header.owner)
            }
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        package.interface_owners[&inner].record,
        PackageInterfaceRecord::RepresentationDeclaration(_)
    ));
    assert!(matches!(
        independently.interfaces[&package.snapshot.root.package_id][&inner],
        PackageInterfaceRecord::RepresentationDeclaration(_)
    ));
    let other = package
        .snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Declaration(record) if record.name.as_str() == "Other" => {
                Some(record.header.owner)
            }
            _ => None,
        })
        .unwrap();
    assert!(!package.interface_owners.contains_key(&other));
}

#[test]
fn owned_record_transport_preserves_supported_graph31_owner_bytes_in_current_source() {
    let original = owned_record_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let mut historical = package
        .snapshot
        .owners
        .values()
        .find(|record| matches!(record, OwnerRecord::Module(_)))
        .unwrap()
        .clone();
    let OwnerRecord::Module(module) = &mut historical else {
        unreachable!()
    };
    module.header.contract_version = 31;
    let (digest, historical_bytes) = encode_owner(&historical).unwrap();
    assert_eq!(&historical_bytes[..8], b"LKJOWN31");
    let owner_key = ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes());
    let mixed = rehash_owner(&original, historical);
    let bytes = mixed.encode().unwrap();
    let decoded = PackageContainer::decode(&bytes, mixed.root.transport).unwrap();
    assert_eq!(decoded.encode().unwrap(), bytes);
    let admitted = decoded.admit().unwrap();
    assert_eq!(admitted.container.objects[&owner_key], historical_bytes);
    let snapshot = &admitted.packages[&admitted.container.root.package_revision].snapshot;
    assert!(
        snapshot
            .owners
            .values()
            .any(|owner| owner.header().contract_version == 31)
    );
    assert!(snapshot.owners.values().any(|owner| matches!(owner,
        OwnerRecord::Declaration(record) if record.header.contract_version == crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            && matches!(record.payload, DeclarationPayload::OwnedRecord { .. }))));
    crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap();
}

#[test]
fn owned_record_unused_body_rejects_cross_module_pack_and_same_shape_foreign_field() {
    let original = owned_record_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let outsider = package
        .snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Module(record) if record.name.as_str() == "outsider" => {
                let OwnerKey::Module(id) = record.header.owner else {
                    unreachable!()
                };
                Some(id)
            }
            _ => None,
        })
        .unwrap();
    let mut make = package
        .snapshot
        .owners
        .values()
        .find(|record| {
            matches!(record,
        OwnerRecord::Declaration(record) if record.name.as_str() == "make")
        })
        .unwrap()
        .clone();
    let OwnerRecord::Declaration(declaration) = &mut make else {
        unreachable!()
    };
    declaration.module = outsider;
    reject_owned_record_transport(&rehash_owner(&original, make));

    let other = package
        .snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Declaration(record) if record.name.as_str() == "Other" => {
                let DeclarationPayload::OwnedRecord { fields, .. } = &record.payload else {
                    unreachable!()
                };
                Some(fields[0])
            }
            _ => None,
        })
        .unwrap();
    let mut pack = package.snapshot.owners.values().find(|record| matches!(record,
        OwnerRecord::Expression(record) if matches!(&record.operation,
            ExpressionOperation::PackOwnedRecord { fields, .. } if fields.len() == 1
                && package.snapshot.owners.get(&OwnerKey::Field(fields[0].field.field))
                    .is_some_and(|field| matches!(field, OwnerRecord::Field(field) if field.name.as_str() == "value"))
        ))).unwrap().clone();
    let OwnerRecord::Expression(record) = &mut pack else {
        unreachable!()
    };
    let ExpressionOperation::PackOwnedRecord { fields, .. } = &mut record.operation else {
        unreachable!()
    };
    fields[0].field.field = other;
    reject_owned_record_transport(&rehash_owner(&original, pack));
}

#[test]
fn owned_record_unused_phantom_arguments_cannot_hide_owned_or_callable_actuals() {
    let original = owned_record_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let unused = package
        .snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Declaration(record) if record.name.as_str() == "unused" => {
                let DeclarationPayload::Function(function) = &record.payload else {
                    unreachable!()
                };
                Some(function.parameters[0])
            }
            _ => None,
        })
        .unwrap();
    let OwnerRecord::Parameter(parameter) = &package.snapshot.owners[&OwnerKey::Parameter(unused)]
    else {
        unreachable!()
    };
    let TypeForm::Applied { declaration, .. } = &package.snapshot.types[&parameter.ty].form else {
        unreachable!()
    };
    let i64_type = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap()).unwrap();
    for actual in [
        TypeForm::ByteBuffer,
        TypeForm::Function {
            parameters: Vec::new(),
            result: i64_type.0,
        },
    ] {
        let actual = encode_type_object(&TypeObject::new(actual).unwrap()).unwrap();
        let applied = encode_type_object(
            &TypeObject::new(TypeForm::Applied {
                declaration: *declaration,
                arguments: vec![actual.0],
            })
            .unwrap(),
        )
        .unwrap();
        let mut replacement = package.snapshot.owners[&OwnerKey::Parameter(unused)].clone();
        let OwnerRecord::Parameter(parameter) = &mut replacement else {
            unreachable!()
        };
        parameter.ty = applied.0;
        let mut hostile = rehash_owner(&original, replacement);
        for (digest, bytes) in [i64_type.clone(), actual, applied] {
            hostile.objects.insert(
                ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
                bytes,
            );
        }
        reject_owned_record_transport(&hostile);
    }
}

#[test]
fn owned_record_unused_intrinsic_result_cannot_manufacture_a_nominal_owner() {
    let original = owned_record_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let envelope_type = package.snapshot.types.iter().find_map(|(digest, object)| match object.form {
        TypeForm::Named { declaration } if package.snapshot.owners.get(&OwnerKey::Declaration(declaration.declaration))
            .is_some_and(|record| matches!(record, OwnerRecord::Declaration(record) if record.name.as_str() == "Envelope")) => Some(*digest),
        _ => None,
    }).unwrap();
    let mut replacement = package
        .snapshot
        .owners
        .values()
        .find(|record| {
            matches!(record,
        OwnerRecord::Declaration(record) if record.name.as_str() == "buffer")
        })
        .unwrap()
        .clone();
    let OwnerRecord::Declaration(record) = &mut replacement else {
        unreachable!()
    };
    let DeclarationPayload::External(external) = &mut record.payload else {
        unreachable!()
    };
    external.result = envelope_type;
    reject_owned_record_transport(&rehash_owner(&original, replacement));
}

#[test]
fn owned_record_unused_private_layout_cannot_hide_a_callable_field() {
    let original = owned_record_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let phantom_field = package
        .snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Declaration(record) if record.name.as_str() == "Phantom" => {
                let DeclarationPayload::OwnedRecord { fields, .. } = &record.payload else {
                    unreachable!()
                };
                Some(fields[0])
            }
            _ => None,
        })
        .unwrap();
    let i64_type = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap()).unwrap();
    let callback = encode_type_object(
        &TypeObject::new(TypeForm::Function {
            parameters: Vec::new(),
            result: i64_type.0,
        })
        .unwrap(),
    )
    .unwrap();
    let mut replacement = package.snapshot.owners[&OwnerKey::Field(phantom_field)].clone();
    let OwnerRecord::Field(field) = &mut replacement else {
        unreachable!()
    };
    field.ty = callback.0;
    let mut hostile = rehash_owner(&original, replacement);
    for (digest, bytes) in [i64_type, callback] {
        hostile.objects.insert(
            ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
            bytes,
        );
    }
    reject_owned_record_transport(&hostile);
}

fn opaque_record_importer(
    seed: &[u8],
    imported: &AdmittedClosure,
    body: &str,
    forbidden: Option<(DeclarationReference, PackageRevisionDigest)>,
) -> AdmittedClosure {
    use crate::platform::change::{AuthoredChange, AuthoredChangeSet, ChangeBudget};
    use crate::platform::publication::PublicationOptions;
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(
        &directory.path().join("opaque-importer"),
        &dependency_copy_empty(seed),
        None,
    )
    .unwrap()
    .repository;
    let binding = imported.container.root;
    let package = &imported.packages[&binding.package_revision];
    repository
        .stage_package_transport(binding.transport, &imported.container.encode().unwrap())
        .unwrap();
    let prepared = repository
        .prepare_authored_change(
            &AuthoredChangeSet {
                base: repository.current().unwrap().head.revision,
                preconditions: Vec::new(),
                budget: ChangeBudget::default(),
                changes: vec![AuthoredChange::AddDependency {
                    package: package.revision.package,
                    semantic_revision: package.revision.revision.revision_id().unwrap(),
                    package_revision: binding.package_revision,
                }],
            },
            PublicationOptions::default(),
        )
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
    let request = format!(
        "request base={}\ndeclarations.begin\n(units (use opaque {} {}) {body})\ndeclarations.end\n",
        repository.current().unwrap().head.revision,
        package.revision.package,
        binding.package_revision,
    );
    let request =
        crate::platform::control::decode_compact_change("opaque.lkjc", request.as_bytes()).unwrap();
    let prepared = repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
    if let Some((forbidden, revision)) = forbidden {
        // Staging the whole closure cannot add a transitive package to authored imports.
        let request = format!(
            "request base={}\ndeclarations.begin\n(units (reference forbidden declaration {}/{}) (module create forbidden (function create unused (visibility private) (effect pure) (parameter create value (type forbidden) (use consume)) (returns Unit) (body (unit)))))\ndeclarations.end\n",
            repository.current().unwrap().head.revision,
            forbidden.package,
            forbidden.declaration
        );
        let request =
            crate::platform::control::decode_compact_change("forbidden.lkjc", request.as_bytes())
                .unwrap();
        let error = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap_err();
        assert!(
            error
                .iter()
                .any(|error| error.code == "change_validate_type_reference"),
            "{error:?}"
        );
        let request = format!(
            "request base={}\ndeclarations.begin\n(units (use forbidden {} {revision}) (module create forbidden (function create unused (visibility private) (effect pure) (parameter create value (type forbidden::Leaf) (use consume)) (returns Unit) (body (unit)))))\ndeclarations.end\n",
            repository.current().unwrap().head.revision,
            forbidden.package
        );
        let request = crate::platform::control::decode_compact_change(
            "forbidden-import.lkjc",
            request.as_bytes(),
        )
        .unwrap();
        let error = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap_err();
        assert!(
            error
                .iter()
                .any(|error| error.code == "change_reference_dependency_missing"),
            "{error:?}"
        );
    }
    repository.export_package_container().unwrap()
}

#[test]
fn opaque_owned_record_transport_admits_transitive_layout_without_authored_import_authority() {
    use crate::platform::kernel::ExpressionRead;
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create leaf (owned-record create Leaf (visibility public) (field create value (type I64)))))\ndeclarations.end\n",
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let leaf = GraphRepository::create(&directory.path().join("leaf"), &snapshot, None)
        .unwrap()
        .repository
        .export_package_container()
        .unwrap();
    let leaf_package = &leaf.packages[&leaf.container.root.package_revision];
    let leaf_declaration = leaf_package
        .interface_owners
        .values()
        .find_map(|owner| {
            matches!(owner.record, PackageInterfaceRecord::Declaration(_)).then_some(owner.owner())
        })
        .unwrap();
    let OwnerKey::Declaration(leaf_id) = leaf_declaration else {
        unreachable!()
    };
    let supplier = opaque_record_importer(
        b"opaque-supplier",
        &leaf,
        "(module create supplier
            (owned-record create Envelope (visibility public) (field create hidden (type opaque::Leaf)))
            (function create relay (visibility public) (effect pure)
              (parameter create packet (type Envelope) (use consume))
              (returns Envelope) (body (local packet))))",
        None,
    );
    let root = opaque_record_importer(
        b"opaque-root",
        &supplier,
        "(module create consumer (function create relay (visibility public) (effect pure)
            (parameter create packet (type opaque::Envelope) (use consume))
            (returns opaque::Envelope) (body (call opaque::relay (local packet)))))",
        Some((
            DeclarationReference {
                package: leaf_package.revision.package,
                declaration: leaf_id,
            },
            leaf.container.root.package_revision,
        )),
    );
    let root_package = &root.packages[&root.container.root.package_revision];
    let supplier_package = &supplier.packages[&supplier.container.root.package_revision];
    assert_eq!(root_package.snapshot.dependencies.len(), 1);
    assert!(
        root_package
            .snapshot
            .has_dependency(supplier_package.revision.package)
            .unwrap()
    );
    assert!(
        !root_package
            .snapshot
            .has_dependency(leaf_package.revision.package)
            .unwrap()
    );
    assert!(
        root_package
            .snapshot
            .has_representation_dependency(leaf_package.revision.package)
            .unwrap()
    );
    assert!(
        root_package
            .snapshot
            .package_interface_owner(leaf_package.revision.package, leaf_declaration)
            .unwrap()
            .is_none()
    );
    assert!(
        root_package
            .snapshot
            .package_representation_owner(leaf_package.revision.package, leaf_declaration)
            .unwrap()
            .is_some()
    );
    let independent =
        crate::platform::package_transport::oracle::reconstruct(&root.container).unwrap();
    let independently_read = &independent.snapshots[&root_package.revision.package];
    assert_eq!(
        independently_read.dependencies,
        root_package.snapshot.dependencies
    );
    assert_eq!(
        independently_read.representation_dependencies,
        root_package.snapshot.representation_dependencies
    );
    assert_eq!(
        independently_read.dependency_interfaces,
        root_package.snapshot.dependency_interfaces
    );
    assert!(crate::platform::kernel::memory_reference::accepts(
        &root_package.snapshot
    ));
}
