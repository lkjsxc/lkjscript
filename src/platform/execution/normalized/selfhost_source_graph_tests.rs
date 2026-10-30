//! Raw source serialization and independent expectations for source closure.
//! No normalized IR, trusted reference list, or validation certificate is exported.
use super::*;

fn word(bytes: &mut [u8], at: usize, value: usize) {
    bytes[at..at + 4].copy_from_slice(&u32::try_from(value).unwrap().to_le_bytes());
}

fn read_word(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

pub(super) fn set_allowance(bytes: &mut [u8], allowance: Option<u64>) {
    word(bytes, 100, usize::from(allowance.is_some()));
    bytes[104..112].copy_from_slice(&allowance.unwrap_or(0).to_le_bytes());
}

pub(super) fn packet(source: &KernelSnapshot) -> Vec<u8> {
    packet_with_commitment(source, kernel::semantic_state_digest(source).unwrap())
}

// The ordinary helper retains the strict independent snapshot oracle. A fixture
// targeting an object/directory binding error must instead supply a commitment
// rebuilt from its actual encoded map rows, because that oracle rejects the
// intended defect before the native reader can observe it.
fn packet_with_commitment(source: &KernelSnapshot, state: kernel::SemanticStateDigest) -> Vec<u8> {
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
    bytes[..8].copy_from_slice(b"LKJSG002");
    word(&mut bytes, 12, source.owners.len());
    word(&mut bytes, 16, source.types.len());
    word(&mut bytes, 20, source.retirements.len());
    word(&mut bytes, 24, name.len());
    bytes[32..48].copy_from_slice(&source.root.package_id.bytes());
    bytes[48..80].copy_from_slice(&state.bytes());
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
    set_allowance(&mut bytes, None);
    bytes
}

// Synthetic capacity fixtures change inventories outside a repository. Rebuild
// both logical commitments exactly as the public authoring result would, before
// using the independent kernel oracle or serializing the carrier.
fn rebind_capacity_inventory(source: &mut KernelSnapshot) {
    let owners = source
        .owners
        .iter()
        .map(|(key, record)| {
            let (object, _) = kernel::encode_owner(record).unwrap();
            (
                kernel::owner_map_key(*key).to_vec(),
                kernel::encode_owner_binding(&kernel::OwnerBinding {
                    kind: record.kind(),
                    object,
                }),
            )
        })
        .collect::<Vec<_>>();
    let retired = source
        .retirements
        .iter()
        .map(|(key, record)| {
            let (object, _) = kernel::encode_retirement(record).unwrap();
            (
                kernel::retirement_map_key(*key).to_vec(),
                object.bytes().to_vec(),
            )
        })
        .collect::<Vec<_>>();
    let owners = crate::platform::persistent_map::MapContentRoot::from_sorted(owners).unwrap();
    let retired = crate::platform::persistent_map::MapContentRoot::from_sorted(retired).unwrap();
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        u64::try_from(source.owners.len()).unwrap(),
        owners.digest(),
    );
    source.root.retirements = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.retirements.page(),
        u64::try_from(source.retirements.len()).unwrap(),
        retired.digest(),
    );
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
    word(bytes, 8, length);
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
fn selfhost_source_graph_native_admits_complete_carriers_above_former_byte_and_proof_limits() {
    const FUNCTIONS: usize = 256;
    const INLINE_BYTES: usize = 64 * 1024;
    let mut proposal = String::from("declarations.begin\n(units (module create carrier-capacity\n");
    for index in 0..FUNCTIONS {
        proposal.push_str(&format!(
            "(function create value-{index} (visibility public) (effect pure) \
             (returns Text) (body (text \"seed\")))\n"
        ));
    }
    proposal.push_str("))\ndeclarations.end\n");
    let mut source = byte_buffer_tests::author_only(&proposal).unwrap();
    let mut literals = 0;
    for owner in source.owners.values_mut() {
        if let OwnerRecord::Expression(kernel::ExpressionRecord {
            operation:
                kernel::ExpressionOperation::Text {
                    value: kernel::TextValue::Inline { text },
                },
            ..
        }) = owner
        {
            *text = "x".repeat(INLINE_BYTES);
            literals += 1;
        }
    }
    assert_eq!(literals, FUNCTIONS);
    rebind_capacity_inventory(&mut source);
    kernel::validate_full(&source).unwrap();
    let bytes = packet(&source);
    assert!(bytes.len() > 8_388_608);
    let obsolete_proof_estimate = 4096
        + 128 * bytes.len()
        + 1024 * (source.owners.len() + source.types.len() + source.retirements.len());
    assert!(obsolete_proof_estimate > i32::MAX as usize);
    assert_eq!(&bytes[100..112], &[0; 12]);
    let carrier_digest = Sha256::digest(&bytes);
    eprintln!(
        "selfhost source carrier capacity entry=source-links::validate engine=self-produced-native owners={} types={} retirements={} bytes={} sha256={} state={}",
        source.owners.len(),
        source.types.len(),
        source.retirements.len(),
        bytes.len(),
        crate::platform::semantic_id::encode_hex(&carrier_digest),
        kernel::semantic_state_digest(&source).unwrap()
    );

    // The cached helper executes the large input only in the self-produced native
    // validator. Its read-graph entry calls source-links::validate on all raw bytes.
    // Rust validation supplies an independent expectation, not semantic input tables.
    emitted_result("read-graph", &bytes, 0);

    let missing = TypeObjectDigest::from_bytes([247; 32]);
    assert!(!source.types.contains_key(&missing));
    let object = kernel::TypeObject::new(kernel::TypeForm::List { item: missing }).unwrap();
    let digest = kernel::encode_type_object(&object).unwrap().0;
    assert!(source.types.insert(digest, object).is_none());
    assert!(kernel::validate_full(&source).is_err());
    let invalid = packet(&source);
    assert!(invalid.len() > 8_388_608);
    assert_eq!(&bytes[48..80], &invalid[48..80]);
    let invalid_digest = Sha256::digest(&invalid);
    assert_ne!(carrier_digest, invalid_digest);
    eprintln!(
        "selfhost source carrier capacity control=unused-missing-type entry=source-links::validate engine=self-produced-native bytes={} sha256={} expected=14",
        invalid.len(),
        crate::platform::semantic_id::encode_hex(&invalid_digest)
    );

    // Fresh canonical object digests and carrier framing reach endpoint admission:
    // read-carrier accepts, while read-graph checks the unused supplied type.
    emitted_result("read-carrier", &invalid, 0);
    emitted_result("read-graph", &invalid, 14);
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
    for tag in [2, u32::MAX as usize] {
        let mut changed = bytes.clone();
        word(&mut changed, 100, tag);
        expect("read-graph", &changed, 12);
    }
    let mut obsolete = bytes.clone();
    obsolete[..8].copy_from_slice(b"LKJSG001");
    expect("read-graph", &obsolete, 12);
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
fn selfhost_source_graph_handles_empty_directories_and_tagged_allowances() {
    let mut source = nominal();
    source.owners.clear();
    source.types.clear();
    source.retirements.clear();
    let empty = packet(&source);
    assert_eq!(read_word(&empty, 12), 0);
    assert_eq!(read_word(&empty, 16), 0);
    assert_eq!(read_word(&empty, 20), 0);
    expect("read-graph", &empty, 0);
    for allowance in [
        None,
        Some(0),
        Some(1),
        Some(u64::from(u32::MAX)),
        Some(u64::MAX),
    ] {
        let mut selected = empty.clone();
        set_allowance(&mut selected, allowance);
        expect("read-graph", &selected, 0);
        let code = if allowance == Some(0) { 6 } else { 0 };
        expect("read-nominal", &selected, code);
        emitted_result("read-nominal", &selected, code);
    }
    let source = nominal();
    let base = packet(&source);
    for (allowance, code) in [(None, 0), (Some(0), 6), (Some(u64::MAX), 0)] {
        let mut selected = base.clone();
        set_allowance(&mut selected, allowance);
        expect("read-graph", &selected, 0);
        expect("read-nominal", &selected, code);
        emitted_result("read-nominal", &selected, code);
    }
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

#[test]
fn selfhost_source_graph_native_admits_carriers_above_former_entity_quota() {
    const RETIREMENTS: usize = 100_001;
    let mut source = nominal();
    let kind = source
        .owners
        .values()
        .find(|owner| matches!(owner, OwnerRecord::Expression(_)))
        .unwrap()
        .kind();
    source.owners.clear();
    source.types.clear();
    source.retirements.clear();
    for index in 1..=RETIREMENTS {
        let mut identity = [0; 16];
        identity[8..].copy_from_slice(&u64::try_from(index).unwrap().to_be_bytes());
        let owner = OwnerKey::Expression(
            crate::platform::semantic_id::ExpressionId::from_bytes(identity).unwrap(),
        );
        source.retirements.insert(
            owner,
            kernel::RetirementRecord {
                graph_contract_version: 1,
                owner,
                last_kind: kind,
                last_name: None,
                last_parent: None,
                last_live_revision: crate::platform::semantic_id::RevisionId::from_digest([31; 32]),
                deletion_change: kernel::ChangeDigest::from_bytes([41; 32]),
            },
        );
    }
    rebind_capacity_inventory(&mut source);
    kernel::validate_full(&source).unwrap();
    let expected = kernel::semantic_state_digest(&source).unwrap();
    assert_eq!(
        kernel::semantic_state_digest_from_root(&source.root).unwrap(),
        expected
    );
    let bytes = packet(&source);
    assert_eq!(read_word(&bytes, 12), 0);
    assert_eq!(read_word(&bytes, 16), 0);
    assert_eq!(read_word(&bytes, 20), RETIREMENTS);
    assert_eq!(&bytes[48..80], &expected.bytes());
    assert_eq!(&bytes[100..112], &[0; 12]);
    let carrier_digest = Sha256::digest(&bytes);
    eprintln!(
        "selfhost source entity capacity entry=source-links::validate engine=self-produced-native owners=0 types=0 retirements={RETIREMENTS} bytes={} sha256={} state={expected} child_deadline_seconds=10",
        bytes.len(),
        crate::platform::semantic_id::encode_hex(&carrier_digest),
    );
    // Reuse the native helper's bounded lifetime and joined child/pipe cleanup.
    // This large carrier is deliberately not executed in either interpreter.
    emitted_result("read-graph", &bytes, 0);

    let changed_owner = OwnerKey::Expression(crate::platform::semantic_id::ExpressionId::migrate(
        b"source-entity-final-binding",
        0,
    ));
    assert_ne!(
        *source.retirements.last_key_value().unwrap().0,
        changed_owner
    );
    source.retirements.last_entry().unwrap().get_mut().owner = changed_owner;
    rebind_capacity_inventory(&mut source);
    assert!(kernel::validate_full(&source).is_err());
    assert_eq!(
        kernel::semantic_state_digest(&source).unwrap_err().code,
        "kernel_state_retirement_key"
    );
    let invalid_state = kernel::semantic_state_digest_from_root(&source.root).unwrap();
    assert_ne!(expected, invalid_state);
    let invalid = packet_with_commitment(&source, invalid_state);
    assert_eq!(invalid.len(), bytes.len());
    assert_eq!(read_word(&invalid, 20), RETIREMENTS);
    assert_eq!(&invalid[48..80], &invalid_state.bytes());
    assert_ne!(carrier_digest, Sha256::digest(&invalid));

    // Independently run the source state engine on the actual carrier directories.
    // This establishes the new logical commitment without asking it to admit the
    // object/directory owner binding, which is the intentionally invalid relation.
    let owner_at = read_word(&invalid, 80);
    let type_at = read_word(&invalid, 84);
    let retirement_at = read_word(&invalid, 88);
    let span_at = read_word(&invalid, 92);
    let mut state_input = vec![0; 32];
    state_input[..16].copy_from_slice(&invalid[32..48]);
    word(&mut state_input, 16, owner_at - 128);
    word(&mut state_input, 20, type_at - owner_at);
    word(&mut state_input, 28, span_at - retirement_at);
    state_input.extend_from_slice(&invalid[128..owner_at]);
    state_input.extend_from_slice(&invalid[owner_at..type_at]);
    state_input.extend_from_slice(&invalid[retirement_at..span_at]);
    emitted_result_bytes("read-state", &state_input, 0, &invalid_state.bytes());

    // The exact same final canonical object and digest admit with its own owner
    // key, then reject with the carrier's different key. Earlier record decoding
    // and digest checks therefore cannot explain this targeted binding refusal.
    let (key, record) = source.retirements.last_key_value().unwrap();
    let (digest, object) = kernel::encode_retirement(record).unwrap();
    let mut binding = kernel::retirement_map_key(record.owner).to_vec();
    binding.extend_from_slice(&digest.bytes());
    binding.extend_from_slice(&object);
    emitted_result_bytes("bind-retirement", &binding, 0, &[]);
    binding[..17].copy_from_slice(&kernel::retirement_map_key(*key));
    emitted_result_bytes("bind-retirement", &binding, 12, &[]);

    // Carrier admission owns the object/directory correspondence check; the graph
    // entry propagates its diagnostic rather than reclassifying it as a link error.
    eprintln!(
        "selfhost source entity capacity control=final-retirement-binding engine=self-produced-native retirements={RETIREMENTS} bytes={} sha256={} state={invalid_state} expected=12",
        invalid.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&invalid)),
    );
    emitted_result("read-carrier", &invalid, 12);
    emitted_result("read-graph", &invalid, 12);
}
