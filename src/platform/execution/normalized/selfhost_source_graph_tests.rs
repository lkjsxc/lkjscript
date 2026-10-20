//! Raw source serialization and independent expectations for source closure.
//! No normalized IR, trusted reference list, or validation certificate is exported.
use super::*;

fn word(bytes: &mut [u8], at: usize, value: usize) {
    bytes[at..at + 4].copy_from_slice(&u32::try_from(value).unwrap().to_le_bytes());
}

fn read_word(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

fn required_fuel(bytes: &[u8]) -> usize {
    4096 + 128 * bytes.len()
        + 1024 * (read_word(bytes, 12) + read_word(bytes, 16) + read_word(bytes, 20))
}

pub(super) fn packet(source: &KernelSnapshot) -> Vec<u8> {
    assert!(source.dependencies.is_empty());
    assert!(source.dependency_types.is_empty());
    assert!(source.blobs.is_empty());
    let mut owners = Vec::new();
    let mut types = Vec::new();
    let mut retirements = Vec::new();
    let mut objects = Vec::new();
    for (key, record) in &source.owners {
        let (digest, bytes) = kernel::encode_owner(record).unwrap();
        owners.extend_from_slice(&kernel::owner_map_key(*key));
        owners.extend_from_slice(&kernel::encode_owner_binding(&kernel::OwnerBinding {
            kind: record.kind(),
            object: digest,
        }));
        objects.push(bytes);
    }
    for (key, object) in &source.types {
        let (digest, bytes) = kernel::encode_type_object(object).unwrap();
        assert_eq!(*key, digest);
        types.extend_from_slice(&digest.bytes());
        objects.push(bytes);
    }
    for (key, record) in &source.retirements {
        let (digest, bytes) = kernel::encode_retirement(record).unwrap();
        retirements.extend_from_slice(&kernel::retirement_map_key(*key));
        retirements.extend_from_slice(&digest.bytes());
        objects.push(bytes);
    }
    let name = source.root.package_name.as_str().as_bytes();
    let mut bytes = vec![0; 128];
    bytes[..8].copy_from_slice(b"LKJSG001");
    word(&mut bytes, 12, source.owners.len());
    word(&mut bytes, 16, source.types.len());
    word(&mut bytes, 20, source.retirements.len());
    word(&mut bytes, 24, name.len());
    bytes[32..48].copy_from_slice(&source.root.package_id.bytes());
    bytes[48..80].copy_from_slice(&kernel::semantic_state_digest(source).unwrap().bytes());
    bytes.extend_from_slice(name);
    for (at, section) in [(80, owners), (84, types), (88, retirements)] {
        let start = bytes.len();
        word(&mut bytes, at, start);
        bytes.extend(section);
    }
    let spans = bytes.len();
    word(&mut bytes, 92, spans);
    bytes.resize(spans + objects.len() * 8, 0);
    let start = bytes.len();
    word(&mut bytes, 96, start);
    for (index, object) in objects.into_iter().enumerate() {
        let start = bytes.len();
        word(&mut bytes, spans + index * 8, start);
        word(&mut bytes, spans + index * 8 + 4, object.len());
        bytes.extend(object);
    }
    let length = bytes.len();
    word(&mut bytes, 8, length);
    let fuel = required_fuel(&bytes);
    word(&mut bytes, 100, fuel);
    bytes
}

fn nominal() -> KernelSnapshot {
    byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-nominal.lkjc"
    ))
    .unwrap()
}

fn expect(name: &str, bytes: &[u8], code: i64) {
    let expected = if code == 0 {
        bytes[48..80].to_vec()
    } else {
        Vec::new()
    };
    assert_eq!(
        result(name, bytes),
        (code, expected),
        "{name}, length={}",
        bytes.len()
    );
}

fn reframe(bytes: &mut [u8]) {
    let length = bytes.len();
    let fuel = required_fuel(bytes);
    word(bytes, 8, length);
    word(bytes, 100, fuel);
}

#[test]
fn selfhost_source_graph_admits_complete_raw_compiler_and_independent_sources() {
    let small = nominal();
    let closure = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-closure.lkjc"
    ))
    .unwrap();
    for source in [&compiler().0, &small, &closure] {
        let bytes = packet(source);
        expect("read-graph", &bytes, 0);
        assert_eq!(
            &bytes[48..80],
            kernel::semantic_state_digest(source).unwrap().bytes()
        );
        eprintln!(
            "selfhost source graph owners={} types={} retirements={} bytes={} sha256={} state={}",
            source.owners.len(),
            source.types.len(),
            source.retirements.len(),
            bytes.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&bytes)),
            kernel::semantic_state_digest(source).unwrap()
        );
    }
}

#[test]
fn selfhost_source_graph_rejects_extent_directory_order_and_object_forgery() {
    let bytes = packet(&nominal());
    for length in [0, 7, 8, 31, 79, 99, 127, 128, bytes.len() - 1] {
        expect("read-graph", &bytes[..length], 12);
    }
    for offset in [0, 8, 24, 32, 48, 80, 84, 88, 92, 96, 104, 127] {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        expect("read-graph", &changed, 12);
    }
    for offset in [12, 16, 20, 24, 80, 84, 88, 92, 96] {
        let mut changed = bytes.clone();
        word(&mut changed, offset, u32::MAX as usize);
        expect("read-graph", &changed, 12);
    }
    let mut changed = bytes.clone();
    changed.push(0);
    reframe(&mut changed);
    expect("read-graph", &changed, 12);
    let mut changed = bytes.clone();
    word(&mut changed, 28, 1);
    expect("read-graph", &changed, 13);
    for fuel in [0, required_fuel(&bytes) - 1, u32::MAX as usize] {
        let mut changed = bytes.clone();
        word(&mut changed, 100, fuel);
        expect("read-graph", &changed, 6);
    }
    for directory in [80, 84] {
        let width = if directory == 80 { 50 } else { 32 };
        let start = read_word(&bytes, directory);
        for duplicate in [false, true] {
            let mut changed = bytes.clone();
            if duplicate {
                changed.copy_within(start..start + width, start + width);
            } else {
                for index in 0..width {
                    changed.swap(start + index, start + width + index);
                }
            }
            expect("read-graph", &changed, 12);
        }
    }
    let spans = read_word(&bytes, 92);
    for offset in [spans, spans + 4, spans + 8] {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        expect("read-graph", &changed, 12);
    }
    let mut changed = bytes.clone();
    let object = read_word(&bytes, 96);
    changed[object + 8] ^= 1;
    expect("read-graph", &changed, 12);
}

#[test]
fn selfhost_source_graph_checks_unused_and_untaken_reference_endpoints() {
    let base = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-closure.lkjc"
    ))
    .unwrap();
    let mut changed = base.clone();
    let missing = crate::platform::semantic_id::ExpressionId::migrate(b"missing-source-graph", 0);
    let untaken = changed
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(kernel::ExpressionRecord {
                operation: kernel::ExpressionOperation::If { when_false, .. },
                ..
            }) => Some(when_false),
            _ => None,
        })
        .unwrap();
    *untaken = missing;
    assert!(kernel::validate_full(&changed).is_err());
    let bytes = packet(&changed);
    expect("read-carrier", &bytes, 0);
    expect("read-graph", &bytes, 14);

    // The carrier has no selected root or reachability exemption. An extra unused
    // expression must have its own exact references checked as well.
    let mut changed = base.clone();
    let id = crate::platform::semantic_id::ExpressionId::migrate(b"unused-source-graph", 0);
    changed.owners.insert(
        OwnerKey::Expression(id),
        OwnerRecord::Expression(
            kernel::ExpressionRecord::new(
                id,
                kernel::ExpressionOperation::Call {
                    function: DeclarationReference {
                        package: changed.root.package_id,
                        declaration: crate::platform::semantic_id::DeclarationId::migrate(
                            b"missing-source-graph",
                            0,
                        ),
                    },
                    type_arguments: Vec::new(),
                    read_scope_arguments: Vec::new(),
                    effect_arguments: Vec::new(),
                    requirement_arguments: Vec::new(),
                    arguments: Vec::new(),
                },
            )
            .unwrap(),
        ),
    );
    assert!(kernel::validate_full(&changed).is_err());
    let bytes = packet(&changed);
    expect("read-carrier", &bytes, 0);
    expect("read-graph", &bytes, 14);
}

#[test]
fn selfhost_source_graph_rejects_missing_types_foreign_packages_and_retired_owners() {
    let base = nominal();
    let mut changed = base.clone();
    let ty = changed
        .owners
        .values()
        .flat_map(OwnerRecord::type_roots)
        .next()
        .unwrap();
    assert!(changed.types.remove(&ty).is_some());
    assert!(kernel::validate_full(&changed).is_err());
    let bytes = packet(&changed);
    expect("read-carrier", &bytes, 0);
    expect("read-graph", &bytes, 14);

    let mut changed = base.clone();
    let target = changed
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(kernel::ExpressionRecord {
                operation: kernel::ExpressionOperation::Call { function, .. },
                ..
            }) => Some(function),
            _ => None,
        })
        .unwrap();
    target.package = kernel::PackageId::migrate(b"foreign-source-graph", 0);
    let bytes = packet(&changed);
    expect("read-carrier", &bytes, 0);
    expect("read-graph", &bytes, 13);

    let mut changed = base;
    let (key, owner) = changed.owners.first_key_value().unwrap();
    let (key, kind) = (*key, owner.kind());
    changed.retirements.insert(
        key,
        kernel::RetirementRecord {
            graph_contract_version: 1,
            owner: key,
            last_kind: kind,
            last_name: None,
            last_parent: None,
            last_live_revision: crate::platform::semantic_id::RevisionId::from_digest([31; 32]),
            deletion_change: kernel::ChangeDigest::from_bytes([41; 32]),
        },
    );
    assert!(kernel::validate_full(&changed).is_err());
    expect("read-graph", &packet(&changed), 14);
}

#[test]
fn selfhost_source_graph_checks_each_independent_owner_relation_family() {
    let source = byte_buffer_tests::author_only(concat!(
        include_str!("../../../../tests/fixtures/selfhost-source-nominal.lkjc"),
        "\n",
        include_str!("../../../../tests/fixtures/selfhost-closure.lkjc"),
        "\ndeclarations.begin\n(units (module create source-target ",
        "(function create entry (visibility public) (effect pure) (returns Text) (body (text \"ok\"))) ",
        "(component create app (visibility public) ",
        "(port create run (type (function () Text)) (function entry)))) ",
        "(target create source-entry (component source-target::app) (runner command) ",
        "(port source-target::app::run)))\ndeclarations.end\n",
    )).unwrap();
    expect("read-graph", &packet(&source), 0);
    let edges = kernel::extract_relations(
        source.root.package_id,
        &source.owners,
        &source.types,
        &source.dependencies,
    )
    .unwrap();
    let mut targets = BTreeMap::new();
    for edge in edges {
        if let kernel::RelationEndpoint::Owner(target) = edge.target
            && target.package == source.root.package_id
            && edge.source != edge.target
            && source.owners.contains_key(&target.owner)
        {
            targets
                .entry(kernel::owner_map_key(target.owner)[0])
                .or_insert(target.owner);
        }
    }
    // Independent owner relations cover modules, declarations, type parameters,
    // fields, cases, parameters, bindings, expressions and ports.
    assert!(targets.len() >= 9, "{targets:?}");
    for (kind, target) in targets {
        let mut changed = source.clone();
        changed.owners.remove(&target).unwrap();
        assert!(
            kernel::validate_full(&changed).is_err(),
            "owner domain {kind}"
        );
        expect("read-graph", &packet(&changed), 14);
    }
}

#[test]
fn selfhost_source_graph_rejects_unreachable_type_edges_and_blob_boundaries() {
    let source = nominal();
    let mut changed = source.clone();
    let object = kernel::TypeObject::new(kernel::TypeForm::List {
        item: TypeObjectDigest::from_bytes([247; 32]),
    })
    .unwrap();
    let digest = kernel::encode_type_object(&object).unwrap().0;
    changed.types.insert(digest, object);
    expect("read-carrier", &packet(&changed), 0);
    expect("read-graph", &packet(&changed), 14);

    let mut changed = source;
    let id = crate::platform::semantic_id::ExpressionId::migrate(b"blob-source-graph", 0);
    changed.owners.insert(
        OwnerKey::Expression(id),
        OwnerRecord::Expression(
            kernel::ExpressionRecord::new(
                id,
                kernel::ExpressionOperation::Text {
                    value: kernel::TextValue::Blob {
                        digest: kernel::BlobObjectDigest::from_bytes([37; 32]),
                        bytes: 4,
                    },
                },
            )
            .unwrap(),
        ),
    );
    expect("read-carrier", &packet(&changed), 0);
    expect("read-graph", &packet(&changed), 13);
}

#[test]
fn selfhost_source_graph_commitments_do_not_misclassify_type_errors_as_admission() {
    let mut source = byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create shape-only \
        (function create value (visibility public) (effect pure) (returns I64) \
        (body (i64 42)))))\ndeclarations.end\n",
    )
    .unwrap();
    let record = source
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(record) => Some(record),
            _ => None,
        })
        .unwrap();
    record.operation = kernel::ExpressionOperation::Unit {};
    assert!(kernel::validate_full(&source).is_err());
    expect("read-graph", &packet(&source), 0);
    // This expected structural success is deliberately not a compiler acceptance.
}

#[test]
fn selfhost_source_graph_handles_empty_directories_and_exact_allowance() {
    let mut source = nominal();
    source.owners.clear();
    source.types.clear();
    source.retirements.clear();
    let empty = packet(&source);
    assert_eq!(read_word(&empty, 12), 0);
    assert_eq!(read_word(&empty, 16), 0);
    assert_eq!(read_word(&empty, 20), 0);
    expect("read-graph", &empty, 0);
    let mut exhausted = empty.clone();
    word(&mut exhausted, 100, required_fuel(&empty) - 1);
    expect("read-graph", &exhausted, 6);
    let mut just_more = empty.clone();
    word(&mut just_more, 100, required_fuel(&empty) + 1);
    expect("read-graph", &just_more, 0);
    let mut permitted = empty;
    word(&mut permitted, 100, i32::MAX as usize);
    expect("read-graph", &permitted, 0);
}

#[test]
fn selfhost_source_graph_rejects_existing_incompatible_declaration_endpoints() {
    let mut source = nominal();
    let record = source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if matches!(record.payload, kernel::DeclarationPayload::Record { .. }) =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap();
    let target = source
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(kernel::ExpressionRecord {
                operation: kernel::ExpressionOperation::Call { function, .. },
                ..
            }) => Some(function),
            _ => None,
        })
        .unwrap();
    // The package, identity and digest are all valid. A record declaration is
    // nevertheless not a callable endpoint.
    target.declaration = record;
    assert!(kernel::validate_full(&source).is_err());
    let bytes = packet(&source);
    expect("read-carrier", &bytes, 0);
    expect("read-graph", &bytes, 14);
}

#[test]
fn selfhost_source_graph_separates_semantic_state_from_valid_unused_type_inventory() {
    let base = nominal();
    let mut empty = base.clone();
    empty.owners.clear();
    empty.types.clear();
    empty.retirements.clear();
    let state = kernel::semantic_state_digest(&empty).unwrap();

    let primitive = kernel::TypeObject::new(kernel::TypeForm::I64).unwrap();
    let digest = kernel::encode_type_object(&primitive).unwrap().0;
    empty.types.insert(digest, primitive);
    let types_only = packet(&empty);
    assert_eq!(kernel::semantic_state_digest(&empty).unwrap(), state);
    expect("read-graph", &types_only, 0);
    assert_ne!(
        packet(&{
            let mut without_types = empty.clone();
            without_types.types.clear();
            without_types
        }),
        types_only
    );

    // Historical parent/revision observations are not current live references.
    empty.types.clear();
    let (key, owner) = base.owners.first_key_value().unwrap();
    empty.retirements.insert(
        *key,
        kernel::RetirementRecord {
            graph_contract_version: 1,
            owner: *key,
            last_kind: owner.kind(),
            last_name: None,
            last_parent: None,
            last_live_revision: crate::platform::semantic_id::RevisionId::from_digest([31; 32]),
            deletion_change: kernel::ChangeDigest::from_bytes([41; 32]),
        },
    );
    let retirement_only = packet(&empty);
    assert_eq!(read_word(&retirement_only, 12), 0);
    assert_eq!(read_word(&retirement_only, 16), 0);
    assert_eq!(read_word(&retirement_only, 20), 1);
    expect("read-graph", &retirement_only, 0);
}

#[test]
fn selfhost_source_graph_visits_the_final_operand_of_the_final_owner() {
    let mut source = nominal();
    let id = crate::platform::semantic_id::ExpressionId::from_bytes([255; 16]).unwrap();
    let first = source
        .owners
        .keys()
        .find_map(|key| match key {
            OwnerKey::Expression(id) => Some(*id),
            _ => None,
        })
        .unwrap();
    let missing = crate::platform::semantic_id::ExpressionId::migrate(b"last-source-operand", 0);
    assert!(!source.owners.contains_key(&OwnerKey::Expression(missing)));
    source.owners.insert(
        OwnerKey::Expression(id),
        OwnerRecord::Expression(
            kernel::ExpressionRecord::new(
                id,
                kernel::ExpressionOperation::Sequence {
                    items: vec![first, missing],
                },
            )
            .unwrap(),
        ),
    );
    assert_eq!(
        source.owners.last_key_value().unwrap().0,
        &OwnerKey::Expression(id)
    );
    let bytes = packet(&source);
    expect("read-carrier", &bytes, 0);
    expect("read-graph", &bytes, 14);
}
