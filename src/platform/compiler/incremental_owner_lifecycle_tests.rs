//! Accepted child evolution must update both exact enclosing units, including removal.
use super::*;
use crate::platform::publication::PreparedPublication;

fn assert_build(
    repository: &GraphRepository,
    prepared: &PreparedPublication,
    expected: BTreeSet<OwnerKey>,
    compiled: u64,
    removed: u64,
) {
    assert_eq!(prepared.compiler_units, expected);
    assert_eq!(
        prepared.receipt.validation.compiler_units_planned,
        expected.len() as u64
    );
    let before = load_current_compilation(repository).unwrap().unwrap();
    repository.publish(prepared).unwrap();
    let incremental = build_incremental(repository, before.digest, prepared).unwrap();
    assert_eq!(incremental.units_compiled, compiled);
    assert_eq!(incremental.units_removed, removed);
    assert_eq!(incremental.work.inventory_bindings, 0);
    assert!(incremental.units_reused >= 1);
    let artifact = link_artifact(repository, incremental.manifest_digest, &[]).unwrap();
    let clean = build_clean(repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    assert_eq!(incremental.manifest_bytes, clean.manifest_bytes);
    assert_eq!(incremental.manifest_digest, clean.manifest_digest);
    let oracle = link_artifact(repository, clean.manifest_digest, &[]).unwrap();
    assert_eq!(artifact.artifact.bytes, oracle.artifact.bytes);
    validate_current_compilation(repository, incremental.manifest_digest).unwrap();
}

fn read(repository: &GraphRepository, owner: OwnerKey) -> OwnerRecord {
    repository
        .view_current()
        .unwrap()
        .owner(owner)
        .unwrap()
        .value
        .unwrap()
}

fn replace(repository: &GraphRepository, record: OwnerRecord) -> PrimitiveEdit {
    PrimitiveEdit::ReplaceOwner {
        expected: encode_owner(&read(repository, record.owner())).unwrap().0,
        record,
    }
}

fn ports(record: &mut OwnerRecord) -> &mut Vec<crate::platform::semantic_id::PortId> {
    let OwnerRecord::Declaration(record) = record else {
        panic!("component")
    };
    let crate::platform::kernel::DeclarationPayload::Component { ports, .. } = &mut record.payload
    else {
        panic!("component ports")
    };
    ports
}

#[test]
fn port_add_rebind_move_delete_and_component_removal_match_clean_artifacts() {
    let fixture = super::incremental_owner_tests::Fixture::new();
    let repository = &fixture.repository;
    let source = format!(
        r#"request base={}
declarations.begin
(units (module create pairs
  (function create one (as $one) (visibility private) (returns I64) (effect pure) (body (i64 1)))
  (function create two (as $two) (visibility private) (returns I64) (effect pure) (body (i64 2)))
  (component create left (as $left) (visibility private)
    (port create fixed (as $left-fixed) (type (function () I64)) (function one)))
  (component create right (as $right) (visibility private)
    (port create fixed (as $right-fixed) (type (function () I64)) (function one)))))
declarations.end
"#,
        repository.current().unwrap().head.revision
    );
    let request =
        crate::platform::control::decode_compact_change("pairs.lkjc", source.as_bytes()).unwrap();
    let initial = repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    let a = initial.allocated["$left"];
    let b = initial.allocated["$right"];
    assert_build(
        repository,
        &initial.publication,
        BTreeSet::from([a, b, initial.allocated["$one"], initial.allocated["$two"]]),
        4,
        0,
    );

    let mut port = read(repository, initial.allocated["$left-fixed"]);
    let id = crate::platform::semantic_id::PortId::migrate(b"moving-extra-port", 0);
    let key = OwnerKey::Port(id);
    let OwnerRecord::Port(record) = &mut port else {
        panic!("port")
    };
    record.header = OwnerHeader::new(key, OwnerKind::Port);
    record.name = Name::new("moving").unwrap();
    let mut left = read(repository, a);
    ports(&mut left).push(id);
    ports(&mut left).sort_unstable();
    let prepared = repository
        .view_current()
        .unwrap()
        .prepare_change(
            vec![
                PrimitiveEdit::InsertOwner { record: port },
                replace(repository, left),
            ],
            PublicationOptions::default(),
        )
        .unwrap();
    assert_build(repository, &prepared, BTreeSet::from([a]), 1, 0);

    let mut port = read(repository, key);
    let OwnerRecord::Port(record) = &mut port else {
        panic!("port")
    };
    let OwnerKey::Declaration(two) = initial.allocated["$two"] else {
        panic!("function")
    };
    record.implementation = crate::platform::kernel::PortImplementation::Function(
        crate::platform::kernel::DeclarationReference {
            package: repository.view_current().unwrap().package(),
            declaration: two,
        },
    );
    let prepared = repository
        .view_current()
        .unwrap()
        .prepare_change(
            vec![replace(repository, port)],
            PublicationOptions::default(),
        )
        .unwrap();
    assert_build(repository, &prepared, BTreeSet::from([a]), 1, 0);

    let mut port = read(repository, key);
    let OwnerRecord::Port(record) = &mut port else {
        panic!("port")
    };
    let OwnerKey::Declaration(destination) = b else {
        panic!("component")
    };
    record.declaration = destination;
    let mut left = read(repository, a);
    ports(&mut left).retain(|current| *current != id);
    let mut right = read(repository, b);
    ports(&mut right).push(id);
    ports(&mut right).sort_unstable();
    let prepared = repository
        .view_current()
        .unwrap()
        .prepare_change(
            vec![
                replace(repository, port),
                replace(repository, left),
                replace(repository, right),
            ],
            PublicationOptions::default(),
        )
        .unwrap();
    assert_build(repository, &prepared, BTreeSet::from([a, b]), 2, 0);

    let mut right = read(repository, b);
    ports(&mut right).retain(|current| *current != id);
    let prepared = repository
        .view_current()
        .unwrap()
        .prepare_change(
            vec![
                PrimitiveEdit::DeleteOwner {
                    owner: key,
                    expected: encode_owner(&read(repository, key)).unwrap().0,
                },
                replace(repository, right),
            ],
            PublicationOptions::default(),
        )
        .unwrap();
    assert_build(repository, &prepared, BTreeSet::from([b]), 1, 0);

    let edits = [b, initial.allocated["$right-fixed"]]
        .into_iter()
        .map(|owner| PrimitiveEdit::DeleteOwner {
            owner,
            expected: encode_owner(&read(repository, owner)).unwrap().0,
        })
        .collect();
    let prepared = repository
        .view_current()
        .unwrap()
        .prepare_change(edits, PublicationOptions::default())
        .unwrap();
    assert_build(repository, &prepared, BTreeSet::from([b]), 0, 1);
}
