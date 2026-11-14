#[test]
fn selfhost_source_revision_identity_matches_independent_current_core() {
    use crate::platform::publication::RevisionCore;
    use crate::platform::semantic_id::{RepositoryId, RevisionId};
    let source = &compiler().0;
    let mut core = RevisionCore {
        contract_version: 1,
        graph_contract_version: 1,
        repository_id: source.root.repository_id,
        parents: vec![],
        semantic_state: kernel::semantic_state_digest(source).unwrap(),
    };
    let mut previous = None;
    for parents in [vec![], vec![RevisionId::from_digest([0; 32])], vec![RevisionId::from_digest([0; 32]), RevisionId::from_digest([255; 32])]] {
        core.parents = parents;
        let expected = core.revision_id().unwrap();
        let bytes = encode(&core);
        assert_eq!(result("read-revision", &bytes), (0, expected.bytes().to_vec()));
        assert_ne!(previous, Some(expected));
        previous = Some(expected);
    }
    let valid = encode(&core);
    for length in 0..valid.len() {
        assert_eq!(result("read-revision", &valid[..length]), (12, vec![]));
    }
    let mut changed = valid.clone(); changed.push(0);
    assert_eq!(result("read-revision", &changed), (12, vec![]));
    for (offset, value) in [(0,2), (1,2), (2,2), (19,3), (20,254), (53,254), (86,9)] {
        changed = valid.clone(); changed[offset] = value;
        assert_eq!(result("read-revision", &changed), (12, vec![]));
    }
    changed = valid.clone(); changed[3..19].fill(0);
    assert_eq!(result("read-revision", &changed), (12, vec![]));
    changed = valid.clone(); changed.splice(0..1, [251,1,0]);
    assert_eq!(result("read-revision", &changed), (12, vec![]));
    core.parents.reverse();
    assert!(core.revision_id().is_err());
    assert_eq!(result("read-revision", &encode(&core)), (12, vec![]));
    core.parents[0] = core.parents[1];
    assert!(core.revision_id().is_err());
    assert_eq!(result("read-revision", &encode(&core)), (12, vec![]));
    core.parents.clear();
    let initial = core.revision_id().unwrap();
    core.repository_id = RepositoryId::migrate(b"source-revision-other-repository", 0);
    let expected = core.revision_id().unwrap();
    assert_ne!(initial, expected);
    assert_eq!(result("read-revision", &encode(&core)), (0, expected.bytes().to_vec()));
    core.semantic_state = kernel::SemanticStateDigest::from_bytes([91;32]);
    let changed_state = core.revision_id().unwrap();
    assert_ne!(expected, changed_state);
    assert_eq!(result("read-revision", &encode(&core)), (0, changed_state.bytes().to_vec()));
}

#[test]
fn selfhost_source_history_records_reject_rehashed_noncanonical_fields() {
    use crate::platform::semantic_id::RevisionId;
    let source = &compiler().0;
    let dependency = kernel::DependencyRecord {
        graph_contract_version: 1,
        package: source.root.package_id,
        semantic_revision: RevisionId::from_digest([31;32]),
        package_revision: kernel::PackageRevisionDigest::from_bytes([47;32]),
    };
    let (_, dependency_bytes) = kernel::encode_dependency(&dependency).unwrap();
    assert_eq!(result("read-dependency", &dependency_bytes), (0, vec![]));
    let retirement = source.retirements.values().next().unwrap().clone();
    for last_name in [None, Some(Name::new("retired_name").unwrap())] {
        for last_parent in [None, Some(retirement.owner)] {
            let mut record = retirement.clone();
            record.last_name = last_name.clone(); record.last_parent = last_parent;
            let (_, bytes) = kernel::encode_retirement(&record).unwrap();
            assert_eq!(result("read-retirement", &bytes), (0, vec![]));
        }
    }
    let mut record = retirement.clone(); record.last_name = None; record.last_parent = None;
    let (_, retirement_bytes) = kernel::encode_retirement(&record).unwrap();
    for (name, magic, context, bytes, edits) in [
        ("read-dependency", b"LKJDEP14", "lkjscript.kernel.dependency-envelope", dependency_bytes, vec![(0,2),(1,17),(18,254),(51,10)]),
        ("read-retirement", b"LKJRET14", "lkjscript.kernel.retirement-envelope", retirement_bytes, vec![(0,2),(1,17),(2,255),(19,29),(20,2),(21,2),(22,254),(55,8)]),
    ] {
        for length in [0, 17, bytes.len()-1] {
            assert_eq!(result(name, &bytes[..length]), (12, vec![]));
        }
        let payload = &bytes[18..bytes.len()-32];
        for (offset, value) in edits {
            let mut changed = payload.to_vec(); changed[offset] = value;
            let bytes = object_envelope(magic, context, &changed);
            if name == "read-dependency" {
                assert!(kernel::decode_dependency(&bytes, &dependency.package, kernel::DependencyObjectDigest::of(&bytes)).is_err());
            } else {
                assert!(kernel::decode_retirement(&bytes, record.owner, kernel::RetirementObjectDigest::of(&bytes)).is_err());
            }
            assert_eq!(result(name, &bytes), (12, vec![]));
        }
        let mut changed = payload.to_vec(); changed.push(0);
        assert_eq!(result(name, &object_envelope(magic, context, &changed)), (12, vec![]));
        let mut changed = bytes; changed[8] = 2;
        assert_eq!(result(name, &changed), (12, vec![]));
    }
}
