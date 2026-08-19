use super::*;
use crate::WorkspaceId;
use crate::release::DecodedRelease;
use crate::release::tests::{consumer_fixture, pair_consumer_fixture, producer_fixture};
use crate::schema::Node;
use std::fs;
use std::path::PathBuf;

struct DiamondFixture {
    root: ReleaseId,
    entry: ReleaseItemId,
    releases: Vec<Vec<u8>>,
    workspace_bytes: Vec<[u8; 16]>,
}

fn bytes(value: &[u8]) -> ApplicationValue {
    ApplicationValue::Bytes(ByteString::from_slice(value).expect("application fixture bytes"))
}

fn export(release: &DecodedRelease, name: &str) -> ReleaseItemId {
    release
        .exports
        .iter()
        .find(|export| export.name == name)
        .expect("fixture export")
        .target
}

fn diamond_fixture() -> DiamondFixture {
    let (shared_snapshot, shared_request) = producer_fixture(0x81, 0, false, false);
    let shared = release::prepare(&shared_snapshot, &shared_request, &[]).expect("shared release");
    let shared_id = shared.release_id();
    let shared_bytes = shared.bytes().to_vec();

    let (left_snapshot, left_request) = consumer_fixture(0x82, "example/app-left", shared_id);
    let left = release::prepare(
        &left_snapshot,
        &left_request,
        std::slice::from_ref(&shared_bytes),
    )
    .expect("left release");
    let (right_snapshot, right_request) = consumer_fixture(0x83, "example/app-right", shared_id);
    let right = release::prepare(
        &right_snapshot,
        &right_request,
        std::slice::from_ref(&shared_bytes),
    )
    .expect("right release");
    let left_bytes = left.bytes().to_vec();
    let right_bytes = right.bytes().to_vec();

    let (root_snapshot, root_request) = pair_consumer_fixture(
        0x84,
        "example/application-root",
        left.release_id(),
        right.release_id(),
        "entry",
        "entry",
        b"abc",
    );
    let dependencies = vec![
        left_bytes.clone(),
        right_bytes.clone(),
        shared_bytes.clone(),
    ];
    let root = release::prepare(&root_snapshot, &root_request, &dependencies)
        .expect("application root release");
    let root_bytes = root.bytes().to_vec();
    let root_decoded = release::decode(&root_bytes).expect("root decode");
    DiamondFixture {
        root: root.release_id(),
        entry: export(&root_decoded, "entry"),
        releases: vec![right_bytes, root_bytes, shared_bytes, left_bytes],
        workspace_bytes: vec![
            shared_snapshot.workspace().as_bytes(),
            left_snapshot.workspace().as_bytes(),
            right_snapshot.workspace().as_bytes(),
            root_snapshot.workspace().as_bytes(),
        ],
    }
}

fn request(fixture: &DiamondFixture) -> ApplicationBuildRequest {
    let entry = ApplicationTarget {
        release: fixture.root,
        item: fixture.entry,
    };
    ApplicationBuildRequest {
        version: APPLICATION_CONTRACT_VERSION,
        root_release: fixture.root,
        entry,
        profile: InvocationProfile::BytesStream,
        policy: RunPolicy {
            fuel: 10_000,
            maximum_frames: 64,
        },
        tests: vec![ApplicationTestCase {
            name: "diamond_entry".into(),
            target: entry,
            arguments: vec![bytes(b"abc")],
            expected: ApplicationTestExpectation::Value(bytes(b"abc")),
            policy: RunPolicy {
                fuel: 10_000,
                maximum_frames: 64,
            },
        }],
    }
}

#[test]
fn bundled_graph_application_is_canonical_offline_and_rejects_v2() {
    let fixture = diamond_fixture();
    let request = request(&fixture);
    let prepared = prepare(&request, &fixture.releases).expect("prepare application");
    let artifact = prepared.bytes().to_vec();
    let inspection = inspect(&artifact).expect("inspect application");
    assert_eq!(inspection.format_version, 5);
    assert_eq!(inspection.root_release, fixture.root);
    assert_eq!(inspection.releases.len(), 4);
    assert_eq!(inspection.graph_edges, 4);
    assert_eq!(inspection.graph_depth, 3);
    assert_eq!(inspection.provenance, "absent");
    for workspace in &fixture.workspace_bytes {
        assert!(
            !artifact
                .windows(WorkspaceId::BYTE_LEN)
                .any(|window| window == workspace)
        );
    }
    let report = test(&artifact).expect("application and release tests");
    assert!(report.all_passed());
    assert_eq!(report.release_total, 4);
    assert_eq!(report.application_total, 1);
    assert_eq!(
        run_stream(&artifact, b"offline").expect("stream"),
        b"offline"
    );
    let run = run(
        &artifact,
        &ApplicationInvocation {
            version: APPLICATION_CONTRACT_VERSION,
            arguments: vec![bytes(b"typed")],
        },
    )
    .expect("typed run");
    assert_eq!(run.result.value, bytes(b"typed"));

    let mut reversed = fixture.releases.clone();
    reversed.reverse();
    assert_eq!(
        prepare(&request, &reversed)
            .expect("permuted graph")
            .bytes(),
        artifact
    );

    for old in [
        b"LKJAPP\0\x02".as_slice(),
        b"LKJAPP\0\x03".as_slice(),
        b"LKJAPP\0\x04".as_slice(),
    ] {
        assert_eq!(
            validate(old).expect_err("old application rejection").code,
            ErrorCode::ArtifactCorrupt
        );
    }
    for end in 0..artifact.len() {
        assert!(validate(&artifact[..end]).is_err(), "truncation {end}");
    }
    let mut trailing = artifact.clone();
    trailing.push(0);
    assert_eq!(
        validate(&trailing).expect_err("trailing").code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn application_v5_profile_and_contract_json_reject_v4_shapes() {
    assert_eq!(
        validate_contract_version(4)
            .expect_err("application contract v4 rejection")
            .code,
        ErrorCode::ProtocolVersion
    );
    assert!(serde_json::from_str::<InvocationProfile>(r#""typed""#).is_err());
    assert_eq!(
        serde_json::from_str::<InvocationProfile>(r#"{"kind":"typed"}"#).expect("tagged profile"),
        InvocationProfile::Typed
    );
    assert!(serde_json::from_str::<InvocationProfile>(r#"{"kind":"typed","extra":0}"#).is_err());
}

#[test]
fn application_value_binary_is_canonical_bounded_and_strict() {
    let fixture = diamond_fixture();
    let target = ApplicationTarget {
        release: fixture.root,
        item: fixture.entry,
    };
    let value = ApplicationValue::Sequence {
        ty: target,
        elements: vec![
            ApplicationValue::Text(TextString::try_from_str("exact text").expect("text")),
            bytes(b"exact bytes"),
            ApplicationValue::Sum {
                ty: target,
                variant: target,
                payload: Some(Box::new(ApplicationValue::I64(-7))),
            },
        ],
    };
    let encoded = encode_application_value_binary(&value, 4096).expect("binary value");
    assert_eq!(
        decode_application_value_binary(&encoded, encoded.len()).expect("decode"),
        value
    );
    assert!(encode_application_value_binary(&value, encoded.len() - 1).is_err());
    assert!(decode_application_value_binary(&encoded, encoded.len() - 1).is_err());
    for end in 0..encoded.len() {
        assert!(decode_application_value_binary(&encoded[..end], 4096).is_err());
    }
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(decode_application_value_binary(&trailing, 4096).is_err());
    assert!(decode_application_value_binary(&[0xff], 4096).is_err());
    assert!(decode_application_value_binary(&[2, 2], 4096).is_err());
}

#[test]
fn application_nominal_values_use_exact_release_identity() {
    let (shared_snapshot, shared_request) = producer_fixture(0x91, 0, false, false);
    let shared = release::prepare(&shared_snapshot, &shared_request, &[]).expect("shared release");
    let shared_bytes = shared.bytes().to_vec();
    let shared_decoded = release::decode(&shared_bytes).expect("shared decode");
    let frame = export(&shared_decoded, "frame");
    let Node::ProductType { fields, .. } = shared_decoded
        .snapshot
        .node(frame.to_local_node().expect("frame local ID"))
        .expect("frame node")
    else {
        panic!("frame export must be a product")
    };
    let field = ReleaseItemId::from_local_node(fields[0]).expect("field local ID");

    let (consumer_snapshot, consumer_request) =
        consumer_fixture(0x92, "example/typed-consumer", shared.release_id());
    let consumer = release::prepare(
        &consumer_snapshot,
        &consumer_request,
        std::slice::from_ref(&shared_bytes),
    )
    .expect("consumer release");
    let consumer_bytes = consumer.bytes().to_vec();
    let consumer_decoded = release::decode(&consumer_bytes).expect("consumer decode");
    let entry = ApplicationTarget {
        release: consumer.release_id(),
        item: export(&consumer_decoded, "frame_passthrough"),
    };
    let frame_target = ApplicationTarget {
        release: shared.release_id(),
        item: frame,
    };
    let value = ApplicationValue::Product {
        ty: frame_target,
        fields: vec![ApplicationFieldValue {
            field: ApplicationTarget {
                release: shared.release_id(),
                item: field,
            },
            value: bytes(b"nominal"),
        }],
    };
    let request = ApplicationBuildRequest {
        version: APPLICATION_CONTRACT_VERSION,
        root_release: consumer.release_id(),
        entry,
        profile: InvocationProfile::Typed,
        policy: RunPolicy {
            fuel: 1_000,
            maximum_frames: 32,
        },
        tests: vec![ApplicationTestCase {
            name: "nominal_round_trip".into(),
            target: entry,
            arguments: vec![value.clone()],
            expected: ApplicationTestExpectation::Value(value.clone()),
            policy: RunPolicy {
                fuel: 1_000,
                maximum_frames: 32,
            },
        }],
    };
    let application = prepare(&request, &[consumer_bytes, shared_bytes]).expect("typed app");
    let receipt = run(
        application.bytes(),
        &ApplicationInvocation {
            version: APPLICATION_CONTRACT_VERSION,
            arguments: vec![value.clone()],
        },
    )
    .expect("nominal run");
    assert_eq!(receipt.result.value, value);
}

#[test]
fn graph_corruption_missing_and_unrelated_inputs_reject_before_application_build() {
    let fixture = diamond_fixture();
    let request = request(&fixture);
    let mut missing = fixture.releases.clone();
    missing.pop();
    assert!(prepare(&request, &missing).is_err());

    let mut corrupt = fixture.releases.clone();
    let last = corrupt.last_mut().expect("release bytes");
    let offset = last.len() / 2;
    last[offset] ^= 1;
    assert_eq!(
        prepare(&request, &corrupt)
            .expect_err("corrupt dependency")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let (unrelated_snapshot, unrelated_request) = producer_fixture(0xa1, 0, false, true);
    let unrelated =
        release::prepare(&unrelated_snapshot, &unrelated_request, &[]).expect("unrelated");
    let mut extra = fixture.releases.clone();
    extra.push(unrelated.bytes().to_vec());
    assert_eq!(
        prepare(&request, &extra)
            .expect_err("unrelated release")
            .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn application_mutation_and_publication_boundaries_are_strict() {
    const CASES: usize = 10_000;
    let fixture = diamond_fixture();
    let prepared = prepare(&request(&fixture), &fixture.releases).expect("application");
    let original = prepared.bytes();
    let mut state = 0xa99a_cafe_2026_0818_u64;
    for case in 0..CASES {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let offset = usize::try_from(state).unwrap_or(0) % original.len();
        let bit = 1_u8 << u32::try_from((state >> 61) & 7).unwrap_or(0);
        let mut mutated = original.to_vec();
        mutated[offset] ^= bit;
        assert!(
            validate(&mutated).is_err(),
            "mutation {case} at {offset} validated"
        );
    }

    let directory = tempfile::tempdir().expect("publication directory");
    let destination = directory.path().join("application.lkja");
    prepared.publish(&destination).expect("publish");
    assert_eq!(read_file(&destination).expect("read"), original);
    assert_eq!(
        prepared
            .publish(&destination)
            .expect_err("no overwrite")
            .code,
        ErrorCode::Io
    );
    for (name, fault) in [
        ("before-write", PublicationFault::BeforeWrite),
        ("after-write", PublicationFault::AfterWrite),
        ("after-file-sync", PublicationFault::AfterFileSync),
    ] {
        let path = directory.path().join(name);
        assert_eq!(
            publish_with_fault(&path, original, fault)
                .expect_err("known failure")
                .code,
            ErrorCode::Io
        );
        assert!(!path.exists());
    }
    for (name, fault) in [
        ("after-link", PublicationFault::AfterLink),
        (
            "after-temporary-removal",
            PublicationFault::AfterTemporaryRemoval,
        ),
        ("after-directory-sync", PublicationFault::AfterDirectorySync),
    ] {
        let path = directory.path().join(name);
        assert_eq!(
            publish_with_fault(&path, original, fault)
                .expect_err("unknown outcome")
                .code,
            ErrorCode::ArtifactPublicationOutcomeUnknown
        );
        assert_eq!(read_file(&path).expect("published after unknown"), original);
    }
    assert!(
        fs::read_dir(directory.path())
            .expect("directory")
            .all(|entry| !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(TEMPORARY_PREFIX))
    );
    for invalid in [
        PathBuf::from("relative.lkja"),
        PathBuf::from(format!("{}/./dot.lkja", directory.path().display())),
        PathBuf::from(format!("{}/../parent.lkja", directory.path().display())),
    ] {
        assert!(prepared.publish(&invalid).is_err());
    }
}
