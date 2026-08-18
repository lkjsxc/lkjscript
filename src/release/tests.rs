use super::*;
use crate::artifact_io::PublicationFault;
use crate::schema::{ByteString, OperationKind, SemanticType, ValueRef};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

fn durable(workspace: WorkspaceId, serial: u64) -> NodeId {
    NodeId::new(workspace, serial).expect("durable fixture ID")
}

fn local(workspace: WorkspaceId, function: u64, ordinal: u32) -> NodeId {
    NodeId::new_function_local(workspace, durable(workspace, function), ordinal)
        .expect("local fixture ID")
}

fn bytes(value: &[u8]) -> RuntimeValue {
    RuntimeValue::Bytes(ByteString::from_slice(value).expect("fixture bytes"))
}

const POLICY: RunPolicy = RunPolicy {
    fuel: 1_000,
    maximum_frames: 32,
};

pub(crate) fn producer_fixture(
    workspace_byte: u8,
    local_offset: u32,
    reverse_functions: bool,
    doubled: bool,
) -> (Snapshot, ReleaseBuildRequest) {
    let workspace = WorkspaceId::from_bytes([workspace_byte; 16]);
    let d = |serial| durable(workspace, serial);
    let l = |function, ordinal| local(workspace, function, local_offset + ordinal);
    let mut functions = vec![d(6), d(8), d(10)];
    if reverse_functions {
        functions.reverse();
    }
    let mut nodes = BTreeMap::from([
        (
            d(1),
            Node::WorkspaceRoot {
                packages: vec![d(2)],
            },
        ),
        (
            d(2),
            Node::Package {
                owner: d(1),
                name: "shared_codec".into(),
                modules: vec![d(3)],
                entry: Some(d(8)),
            },
        ),
        (
            d(3),
            Node::Module {
                owner: d(2),
                name: "codec".into(),
                types: vec![d(4)],
                functions,
            },
        ),
        (
            d(4),
            Node::ProductType {
                owner: d(3),
                name: "frame".into(),
                fields: vec![d(5)],
            },
        ),
        (
            d(5),
            Node::ProductField {
                owner: d(4),
                ordinal: 0,
                name: "payload".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            d(6),
            Node::Function {
                owner: d(3),
                name: "private_canonicalize".into(),
                parameters: vec![d(7)],
                result: SemanticType::Bytes,
                body: Some(l(6, 1)),
            },
        ),
        (
            d(7),
            Node::Parameter {
                owner: d(6),
                ordinal: 0,
                name: "input".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            d(8),
            Node::Function {
                owner: d(3),
                name: "normalize".into(),
                parameters: vec![d(9)],
                result: SemanticType::Bytes,
                body: Some(l(8, 1)),
            },
        ),
        (
            d(9),
            Node::Parameter {
                owner: d(8),
                ordinal: 0,
                name: "input".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            d(10),
            Node::Function {
                owner: d(3),
                name: "inspect_length".into(),
                parameters: vec![d(11)],
                result: SemanticType::I64,
                body: Some(l(10, 1)),
            },
        ),
        (
            d(11),
            Node::Parameter {
                owner: d(10),
                ordinal: 0,
                name: "input".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            l(6, 1),
            Node::Region {
                owner: d(6),
                blocks: vec![l(6, 2)],
            },
        ),
        (
            l(6, 2),
            Node::Block {
                owner: l(6, 1),
                arguments: Vec::new(),
                operations: Vec::new(),
                terminator: Some(l(6, 3)),
            },
        ),
        (
            l(6, 3),
            Node::Operation {
                owner: l(6, 2),
                operation: OperationKind::Return {
                    value: ValueRef::FunctionParameter(d(7)),
                },
            },
        ),
        (
            l(10, 1),
            Node::Region {
                owner: d(10),
                blocks: vec![l(10, 2)],
            },
        ),
        (
            l(10, 2),
            Node::Block {
                owner: l(10, 1),
                arguments: Vec::new(),
                operations: vec![l(10, 3)],
                terminator: Some(l(10, 4)),
            },
        ),
        (
            l(10, 3),
            Node::Operation {
                owner: l(10, 2),
                operation: OperationKind::BytesLen {
                    value: ValueRef::FunctionParameter(d(11)),
                },
            },
        ),
        (
            l(10, 4),
            Node::Operation {
                owner: l(10, 2),
                operation: OperationKind::Return {
                    value: ValueRef::OperationResult {
                        operation: l(10, 3),
                        output: 0,
                    },
                },
            },
        ),
    ]);
    let mut normalize_operations = vec![l(8, 3)];
    let normalize_result = if doubled {
        normalize_operations.push(l(8, 4));
        ValueRef::OperationResult {
            operation: l(8, 4),
            output: 0,
        }
    } else {
        ValueRef::OperationResult {
            operation: l(8, 3),
            output: 0,
        }
    };
    let terminator = if doubled { l(8, 5) } else { l(8, 4) };
    nodes.extend([
        (
            l(8, 1),
            Node::Region {
                owner: d(8),
                blocks: vec![l(8, 2)],
            },
        ),
        (
            l(8, 2),
            Node::Block {
                owner: l(8, 1),
                arguments: Vec::new(),
                operations: normalize_operations,
                terminator: Some(terminator),
            },
        ),
        (
            l(8, 3),
            Node::Operation {
                owner: l(8, 2),
                operation: OperationKind::Call {
                    function: d(6),
                    arguments: vec![ValueRef::FunctionParameter(d(9))],
                },
            },
        ),
        (
            terminator,
            Node::Operation {
                owner: l(8, 2),
                operation: OperationKind::Return {
                    value: normalize_result,
                },
            },
        ),
    ]);
    if doubled {
        nodes.insert(
            l(8, 4),
            Node::Operation {
                owner: l(8, 2),
                operation: OperationKind::BytesConcat {
                    lhs: ValueRef::OperationResult {
                        operation: l(8, 3),
                        output: 0,
                    },
                    rhs: ValueRef::OperationResult {
                        operation: l(8, 3),
                        output: 0,
                    },
                },
            },
        );
    }
    let snapshot = Snapshot::from_parts(
        workspace,
        Revision::new(7),
        d(1),
        12,
        BTreeSet::new(),
        nodes,
    )
    .expect("producer fixture");
    let expected: &[u8] = if doubled { b"abcabc" } else { b"abc" };
    let mut exports = vec![
        ReleaseExportRequest {
            name: "frame".into(),
            target: d(4),
        },
        ReleaseExportRequest {
            name: "inspect_length".into(),
            target: d(10),
        },
        ReleaseExportRequest {
            name: "normalize".into(),
            target: d(8),
        },
    ];
    if reverse_functions {
        exports.reverse();
    }
    let request = ReleaseBuildRequest {
        version: RELEASE_CONTRACT_VERSION,
        workspace,
        revision: snapshot.revision(),
        root: d(2),
        coordinate: "example/shared-codec".into(),
        user_version: if doubled { "2.0.0" } else { "1.0.0" }.into(),
        exports,
        dependencies: Vec::new(),
        imports: Vec::new(),
        tests: vec![ReleaseTestCase {
            name: "normalize_bytes".into(),
            target: d(8),
            arguments: vec![bytes(b"abc")],
            expected: ReleaseTestExpectation::Value(bytes(expected)),
            policy: POLICY,
        }],
    };
    (snapshot, request)
}

pub(crate) fn consumer_fixture(
    workspace_byte: u8,
    coordinate: &str,
    dependency: ReleaseId,
) -> (Snapshot, ReleaseBuildRequest) {
    let workspace = WorkspaceId::from_bytes([workspace_byte; 16]);
    let d = |serial| durable(workspace, serial);
    let l = |function, ordinal| local(workspace, function, ordinal);
    let nodes = BTreeMap::from([
        (
            d(1),
            Node::WorkspaceRoot {
                packages: vec![d(2)],
            },
        ),
        (
            d(2),
            Node::Package {
                owner: d(1),
                name: "consumer".into(),
                modules: vec![d(3)],
                entry: Some(d(10)),
            },
        ),
        (
            d(3),
            Node::Module {
                owner: d(2),
                name: "main".into(),
                types: vec![d(4)],
                functions: vec![d(6), d(8), d(10)],
            },
        ),
        (
            d(4),
            Node::ProductType {
                owner: d(3),
                name: "shared_frame".into(),
                fields: vec![d(5)],
            },
        ),
        (
            d(5),
            Node::ProductField {
                owner: d(4),
                ordinal: 0,
                name: "payload".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            d(6),
            Node::Function {
                owner: d(3),
                name: "shared_normalize".into(),
                parameters: vec![d(7)],
                result: SemanticType::Bytes,
                body: None,
            },
        ),
        (
            d(7),
            Node::Parameter {
                owner: d(6),
                ordinal: 0,
                name: "input".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            d(8),
            Node::Function {
                owner: d(3),
                name: "frame_passthrough".into(),
                parameters: vec![d(9)],
                result: SemanticType::Nominal(d(4)),
                body: Some(l(8, 1)),
            },
        ),
        (
            d(9),
            Node::Parameter {
                owner: d(8),
                ordinal: 0,
                name: "frame".into(),
                ty: SemanticType::Nominal(d(4)),
            },
        ),
        (
            d(10),
            Node::Function {
                owner: d(3),
                name: "entry".into(),
                parameters: vec![d(11)],
                result: SemanticType::Bytes,
                body: Some(l(10, 1)),
            },
        ),
        (
            d(11),
            Node::Parameter {
                owner: d(10),
                ordinal: 0,
                name: "input".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            l(8, 1),
            Node::Region {
                owner: d(8),
                blocks: vec![l(8, 2)],
            },
        ),
        (
            l(8, 2),
            Node::Block {
                owner: l(8, 1),
                arguments: Vec::new(),
                operations: Vec::new(),
                terminator: Some(l(8, 3)),
            },
        ),
        (
            l(8, 3),
            Node::Operation {
                owner: l(8, 2),
                operation: OperationKind::Return {
                    value: ValueRef::FunctionParameter(d(9)),
                },
            },
        ),
        (
            l(10, 1),
            Node::Region {
                owner: d(10),
                blocks: vec![l(10, 2)],
            },
        ),
        (
            l(10, 2),
            Node::Block {
                owner: l(10, 1),
                arguments: Vec::new(),
                operations: vec![l(10, 3)],
                terminator: Some(l(10, 4)),
            },
        ),
        (
            l(10, 3),
            Node::Operation {
                owner: l(10, 2),
                operation: OperationKind::Call {
                    function: d(6),
                    arguments: vec![ValueRef::FunctionParameter(d(11))],
                },
            },
        ),
        (
            l(10, 4),
            Node::Operation {
                owner: l(10, 2),
                operation: OperationKind::Return {
                    value: ValueRef::OperationResult {
                        operation: l(10, 3),
                        output: 0,
                    },
                },
            },
        ),
    ]);
    let snapshot = Snapshot::from_parts(
        workspace,
        Revision::new(3),
        d(1),
        12,
        BTreeSet::new(),
        nodes,
    )
    .expect("consumer fixture");
    let request = ReleaseBuildRequest {
        version: RELEASE_CONTRACT_VERSION,
        workspace,
        revision: snapshot.revision(),
        root: d(2),
        coordinate: coordinate.into(),
        user_version: "1.0.0".into(),
        exports: vec![
            ReleaseExportRequest {
                name: "entry".into(),
                target: d(10),
            },
            ReleaseExportRequest {
                name: "frame_passthrough".into(),
                target: d(8),
            },
        ],
        dependencies: vec![ReleaseDependencyRequest {
            slot: "shared".into(),
            release: dependency,
        }],
        imports: vec![
            ReleaseImportRequest {
                local: d(4),
                dependency_slot: "shared".into(),
                export: "frame".into(),
            },
            ReleaseImportRequest {
                local: d(6),
                dependency_slot: "shared".into(),
                export: "normalize".into(),
            },
        ],
        tests: vec![ReleaseTestCase {
            name: "consumer_entry".into(),
            target: d(10),
            arguments: vec![bytes(b"abc")],
            expected: ReleaseTestExpectation::Value(bytes(b"abc")),
            policy: POLICY,
        }],
    };
    (snapshot, request)
}

pub(crate) fn pair_consumer_fixture(
    workspace_byte: u8,
    coordinate: &str,
    left: ReleaseId,
    right: ReleaseId,
    left_export: &str,
    right_export: &str,
    expected: &[u8],
) -> (Snapshot, ReleaseBuildRequest) {
    let workspace = WorkspaceId::from_bytes([workspace_byte; 16]);
    let d = |serial| durable(workspace, serial);
    let l = |ordinal| local(workspace, 8, ordinal);
    let nodes = BTreeMap::from([
        (
            d(1),
            Node::WorkspaceRoot {
                packages: vec![d(2)],
            },
        ),
        (
            d(2),
            Node::Package {
                owner: d(1),
                name: "pair".into(),
                modules: vec![d(3)],
                entry: Some(d(8)),
            },
        ),
        (
            d(3),
            Node::Module {
                owner: d(2),
                name: "main".into(),
                types: Vec::new(),
                functions: vec![d(4), d(6), d(8)],
            },
        ),
        (
            d(4),
            Node::Function {
                owner: d(3),
                name: "left".into(),
                parameters: vec![d(5)],
                result: SemanticType::Bytes,
                body: None,
            },
        ),
        (
            d(5),
            Node::Parameter {
                owner: d(4),
                ordinal: 0,
                name: "input".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            d(6),
            Node::Function {
                owner: d(3),
                name: "right".into(),
                parameters: vec![d(7)],
                result: SemanticType::Bytes,
                body: None,
            },
        ),
        (
            d(7),
            Node::Parameter {
                owner: d(6),
                ordinal: 0,
                name: "input".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            d(8),
            Node::Function {
                owner: d(3),
                name: "entry".into(),
                parameters: vec![d(9)],
                result: SemanticType::Bytes,
                body: Some(l(1)),
            },
        ),
        (
            d(9),
            Node::Parameter {
                owner: d(8),
                ordinal: 0,
                name: "input".into(),
                ty: SemanticType::Bytes,
            },
        ),
        (
            l(1),
            Node::Region {
                owner: d(8),
                blocks: vec![l(2)],
            },
        ),
        (
            l(2),
            Node::Block {
                owner: l(1),
                arguments: Vec::new(),
                operations: vec![l(3), l(4)],
                terminator: Some(l(5)),
            },
        ),
        (
            l(3),
            Node::Operation {
                owner: l(2),
                operation: OperationKind::Call {
                    function: d(4),
                    arguments: vec![ValueRef::FunctionParameter(d(9))],
                },
            },
        ),
        (
            l(4),
            Node::Operation {
                owner: l(2),
                operation: OperationKind::Call {
                    function: d(6),
                    arguments: vec![ValueRef::OperationResult {
                        operation: l(3),
                        output: 0,
                    }],
                },
            },
        ),
        (
            l(5),
            Node::Operation {
                owner: l(2),
                operation: OperationKind::Return {
                    value: ValueRef::OperationResult {
                        operation: l(4),
                        output: 0,
                    },
                },
            },
        ),
    ]);
    let snapshot = Snapshot::from_parts(
        workspace,
        Revision::new(5),
        d(1),
        10,
        BTreeSet::new(),
        nodes,
    )
    .expect("pair fixture");
    let request = ReleaseBuildRequest {
        version: RELEASE_CONTRACT_VERSION,
        workspace,
        revision: snapshot.revision(),
        root: d(2),
        coordinate: coordinate.into(),
        user_version: "1.0.0".into(),
        exports: vec![ReleaseExportRequest {
            name: "entry".into(),
            target: d(8),
        }],
        dependencies: vec![
            ReleaseDependencyRequest {
                slot: "left".into(),
                release: left,
            },
            ReleaseDependencyRequest {
                slot: "right".into(),
                release: right,
            },
        ],
        imports: vec![
            ReleaseImportRequest {
                local: d(4),
                dependency_slot: "left".into(),
                export: left_export.into(),
            },
            ReleaseImportRequest {
                local: d(6),
                dependency_slot: "right".into(),
                export: right_export.into(),
            },
        ],
        tests: vec![ReleaseTestCase {
            name: "pair_entry".into(),
            target: d(8),
            arguments: vec![bytes(b"abc")],
            expected: ReleaseTestExpectation::Value(bytes(expected)),
            policy: POLICY,
        }],
    };
    (snapshot, request)
}

fn export(release: &DecodedRelease, name: &str) -> ReleaseItemId {
    release
        .exports
        .iter()
        .find(|export| export.name == name)
        .expect("named export")
        .target
}

fn synthetic_release(
    template: &DecodedRelease,
    ordinal: usize,
    dependencies: impl IntoIterator<Item = ReleaseId>,
) -> DecodedRelease {
    let mut identity = [0_u8; 32];
    identity[..8].copy_from_slice(
        &u64::try_from(ordinal.saturating_add(1))
            .expect("synthetic release ordinal")
            .to_le_bytes(),
    );
    let mut release = template.clone();
    release.id = ReleaseId::from_bytes(identity);
    release.bytes = u64::try_from(ordinal)
        .expect("synthetic release byte ordinal")
        .to_le_bytes()
        .to_vec();
    release.dependencies = dependencies
        .into_iter()
        .enumerate()
        .map(|(index, dependency)| ReleaseDependency {
            slot: format!("dependency-{index:03}"),
            release: dependency,
        })
        .collect();
    release.imports.clear();
    release
}

fn synthetic_id(ordinal: usize) -> ReleaseId {
    let mut identity = [0_u8; 32];
    identity[..8].copy_from_slice(
        &u64::try_from(ordinal.saturating_add(1))
            .expect("synthetic release ordinal")
            .to_le_bytes(),
    );
    ReleaseId::from_bytes(identity)
}

fn synthetic_chain(
    template: &DecodedRelease,
    node_count: usize,
) -> (DecodedRelease, Vec<DecodedRelease>) {
    let mut releases = (0..node_count)
        .map(|index| {
            synthetic_release(
                template,
                index,
                (index + 1 < node_count).then(|| synthetic_id(index + 1)),
            )
        })
        .collect::<Vec<_>>();
    let root = releases.remove(0);
    (root, releases)
}

#[test]
fn canonical_release_is_workspace_independent_strict_and_private() {
    assert!(serde_json::from_str::<ReleaseItemId>("0").is_err());
    assert!(serde_json::from_str::<ReleaseItemId>("9223372036854775808").is_err());
    let (first_snapshot, first_request) = producer_fixture(0x11, 0, false, false);
    let (second_snapshot, second_request) = producer_fixture(0x22, 40, true, false);
    let first = prepare(&first_snapshot, &first_request, &[]).expect("first release");
    let second = prepare(&second_snapshot, &second_request, &[]).expect("second release");
    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(first.release_id(), second.release_id());
    assert!(
        !first
            .bytes()
            .windows(WorkspaceId::BYTE_LEN)
            .any(|window| window == first_snapshot.workspace().as_bytes())
    );
    let inspection = inspect(first.bytes()).expect("inspection");
    assert_eq!(
        inspection
            .exports
            .iter()
            .map(|export| export.name.as_str())
            .collect::<Vec<_>>(),
        ["frame", "inspect_length", "normalize"]
    );
    assert!(inspection.private_durable_items > 0);
    assert!(
        test(first.bytes(), &[])
            .expect("release tests")
            .all_passed()
    );

    for end in 0..first.bytes().len() {
        assert!(validate(&first.bytes()[..end]).is_err(), "truncation {end}");
    }
    let mut trailing = first.bytes().to_vec();
    trailing.push(0);
    assert_eq!(
        validate(&trailing).expect_err("trailing bytes").code,
        ErrorCode::ArtifactCorrupt
    );
    let mut old = first.bytes().to_vec();
    old[8..10].copy_from_slice(&0_u16.to_le_bytes());
    assert_eq!(
        validate(&old).expect_err("old version").code,
        ErrorCode::ArtifactCorrupt
    );
    let payload_length_offset = codec::RELEASE_MAGIC.len() + 2 + crate::artifact::SCHEMA_ID.0.len();
    let mut oversized_payload = first.bytes().to_vec();
    oversized_payload[payload_length_offset..payload_length_offset + 8].copy_from_slice(
        &u64::try_from(MAXIMUM_RELEASE_ARTIFACT_BYTES + 1)
            .expect("release artifact byte policy")
            .to_le_bytes(),
    );
    assert_eq!(
        validate(&oversized_payload)
            .expect_err("oversized declared payload")
            .code,
        ErrorCode::PolicyExceeded
    );

    assert!(validate_coordinate(&"a".repeat(MAXIMUM_RELEASE_COORDINATE_BYTES)).is_ok());
    assert!(validate_coordinate(&"a".repeat(MAXIMUM_RELEASE_COORDINATE_BYTES + 1)).is_err());
    assert!(validate_user_version(&"v".repeat(MAXIMUM_RELEASE_VERSION_BYTES)).is_ok());
    assert!(validate_user_version(&"v".repeat(MAXIMUM_RELEASE_VERSION_BYTES + 1)).is_err());
    assert!(
        validate_symbol(
            &"n".repeat(MAXIMUM_RELEASE_NAME_BYTES),
            MAXIMUM_RELEASE_NAME_BYTES,
            "release name"
        )
        .is_ok()
    );
    assert!(
        validate_symbol(
            &"n".repeat(MAXIMUM_RELEASE_NAME_BYTES + 1),
            MAXIMUM_RELEASE_NAME_BYTES,
            "release name"
        )
        .is_err()
    );
    assert!(
        validate_symbol(
            &"s".repeat(MAXIMUM_RELEASE_SLOT_BYTES),
            MAXIMUM_RELEASE_SLOT_BYTES,
            "dependency slot"
        )
        .is_ok()
    );
    assert!(
        validate_symbol(
            &"s".repeat(MAXIMUM_RELEASE_SLOT_BYTES + 1),
            MAXIMUM_RELEASE_SLOT_BYTES,
            "dependency slot"
        )
        .is_err()
    );

    let mut state = 0x771e_a5e0_2026_0818_u64;
    for case in 0..10_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let offset = usize::try_from(state).unwrap_or(0) % first.bytes().len();
        let bit = 1_u8 << u32::try_from((state >> 61) & 7).unwrap_or(0);
        let mut mutated = first.bytes().to_vec();
        mutated[offset] ^= bit;
        assert!(
            validate(&mutated).is_err(),
            "mutation {case} at {offset} validated"
        );
    }
}

#[test]
fn exact_release_graph_limits_accept_boundaries_and_reject_one_over() {
    let (snapshot, request) = producer_fixture(0x18, 0, false, false);
    let prepared = prepare(&snapshot, &request, &[]).expect("graph limit template");
    let template = decode(prepared.bytes()).expect("decoded graph limit template");

    let leaf_ids = (1..MAXIMUM_RELEASE_GRAPH_NODES)
        .map(synthetic_id)
        .collect::<Vec<_>>();
    let root = synthetic_release(&template, 0, leaf_ids.iter().copied());
    let leaves = (1..MAXIMUM_RELEASE_GRAPH_NODES)
        .map(|index| synthetic_release(&template, index, []))
        .collect::<Vec<_>>();
    let boundary = graph::ReleaseGraph::new(root, leaves).expect("graph node boundary");
    assert_eq!(boundary.releases().count(), MAXIMUM_RELEASE_GRAPH_NODES);

    let one_over_ids = (1..=MAXIMUM_RELEASE_GRAPH_NODES)
        .map(synthetic_id)
        .collect::<Vec<_>>();
    let root = synthetic_release(&template, 0, one_over_ids.iter().copied());
    let leaves = (1..=MAXIMUM_RELEASE_GRAPH_NODES)
        .map(|index| synthetic_release(&template, index, []))
        .collect::<Vec<_>>();
    assert_eq!(
        graph::ReleaseGraph::new(root, leaves)
            .expect_err("graph node one-over")
            .code,
        ErrorCode::PolicyExceeded
    );

    let a_ids = (1..=64).map(synthetic_id).collect::<Vec<_>>();
    let b_ids = (65..=127).map(synthetic_id).collect::<Vec<_>>();
    let root = synthetic_release(&template, 0, a_ids.iter().copied());
    let mut supplied = (1..=64)
        .map(|index| synthetic_release(&template, index, b_ids.iter().copied()))
        .collect::<Vec<_>>();
    supplied.extend((65..=127).map(|index| synthetic_release(&template, index, [])));
    let boundary = graph::ReleaseGraph::new(root, supplied).expect("graph edge boundary");
    assert_eq!(boundary.edge_count(), MAXIMUM_RELEASE_GRAPH_EDGES);
    assert_eq!(boundary.depth(), 3);

    let b_ids = (65..=128).map(synthetic_id).collect::<Vec<_>>();
    let root = synthetic_release(&template, 0, a_ids.iter().copied());
    let mut supplied = (1..=64)
        .map(|index| {
            let count = if index == 1 { 64 } else { 63 };
            synthetic_release(&template, index, b_ids.iter().copied().take(count))
        })
        .collect::<Vec<_>>();
    supplied.extend((65..=128).map(|index| synthetic_release(&template, index, [])));
    assert_eq!(
        graph::ReleaseGraph::new(root, supplied)
            .expect_err("graph edge one-over")
            .code,
        ErrorCode::PolicyExceeded
    );

    let (root, supplied) = synthetic_chain(&template, MAXIMUM_RELEASE_GRAPH_DEPTH);
    let boundary = graph::ReleaseGraph::new(root, supplied).expect("graph depth boundary");
    assert_eq!(boundary.depth(), MAXIMUM_RELEASE_GRAPH_DEPTH);
    let (root, supplied) = synthetic_chain(&template, MAXIMUM_RELEASE_GRAPH_DEPTH + 1);
    assert_eq!(
        graph::ReleaseGraph::new(root, supplied)
            .expect_err("graph depth one-over")
            .code,
        ErrorCode::PolicyExceeded
    );

    assert_eq!(
        graph::checked_graph_bytes(MAXIMUM_RELEASE_GRAPH_BYTES - 1, 1)
            .expect("graph byte boundary"),
        MAXIMUM_RELEASE_GRAPH_BYTES
    );
    assert_eq!(
        graph::checked_graph_bytes(MAXIMUM_RELEASE_GRAPH_BYTES, 1)
            .expect_err("graph byte one-over")
            .code,
        ErrorCode::PolicyExceeded
    );
    assert_eq!(
        graph::checked_graph_bytes(usize::MAX, 1)
            .expect_err("graph byte arithmetic overflow")
            .code,
        ErrorCode::PolicyExceeded
    );

    let shared = synthetic_release(&template, 1, []);
    let root = synthetic_release(&template, 0, [shared.id]);
    let duplicate_graph =
        graph::ReleaseGraph::new(root.clone(), vec![shared.clone(), shared.clone()])
            .expect("equal exact release duplicates");
    assert_eq!(duplicate_graph.releases().count(), 2);
    let mut conflicting = shared.clone();
    conflicting.bytes.push(0xff);
    assert_eq!(
        graph::ReleaseGraph::new(root, vec![shared, conflicting])
            .expect_err("conflicting bytes for one exact release identity")
            .code,
        ErrorCode::ArtifactCorrupt
    );

    let self_id = synthetic_id(0);
    let self_dependent = synthetic_release(&template, 0, [self_id]);
    assert_eq!(
        graph::ReleaseGraph::new(self_dependent, Vec::new())
            .expect_err("self dependency")
            .code,
        ErrorCode::ArtifactCorrupt
    );
    let first_id = synthetic_id(0);
    let second_id = synthetic_id(1);
    let first = synthetic_release(&template, 0, [second_id]);
    let second = synthetic_release(&template, 1, [first_id]);
    assert_eq!(
        graph::ReleaseGraph::new(first, vec![second])
            .expect_err("dependency cycle")
            .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn release_publication_and_hostile_paths_are_strict() {
    let (snapshot, request) = producer_fixture(0x19, 0, false, false);
    let prepared = prepare(&snapshot, &request, &[]).expect("prepared release");
    let original = prepared.bytes();
    let directory = tempfile::tempdir().expect("publication directory");

    let destination = directory.path().join("release.lkjr");
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
                .starts_with(RELEASE_TEMPORARY_PREFIX))
    );

    let linked = directory.path().join("linked.lkjr");
    symlink(&destination, &linked).expect("release symlink");
    assert_eq!(
        read_file(&linked).expect_err("symlink input").code,
        ErrorCode::Io
    );
    let parent_link = directory.path().join("linked-parent");
    symlink(directory.path(), &parent_link).expect("parent symlink");
    assert_eq!(
        prepared
            .publish(&parent_link.join("through-parent.lkjr"))
            .expect_err("symlink parent")
            .code,
        ErrorCode::Io
    );
    assert_eq!(
        read_file(directory.path())
            .expect_err("non-regular input")
            .code,
        ErrorCode::Io
    );
    for invalid in [
        PathBuf::from("relative.lkjr"),
        PathBuf::from(format!("{}/./dot.lkjr", directory.path().display())),
        PathBuf::from(format!("{}/../parent.lkjr", directory.path().display())),
    ] {
        assert!(prepared.publish(&invalid).is_err());
    }
}

#[test]
fn two_consumers_share_one_exact_release_and_private_access_rejects() {
    let (producer_snapshot, producer_request) = producer_fixture(0x31, 0, false, false);
    let producer = prepare(&producer_snapshot, &producer_request, &[]).expect("producer");
    let producer_bytes = producer.bytes().to_vec();

    let (left_snapshot, left_request) =
        consumer_fixture(0x41, "example/consumer-normalizer", producer.release_id());
    let left = prepare(
        &left_snapshot,
        &left_request,
        std::slice::from_ref(&producer_bytes),
    )
    .expect("left consumer");
    let (right_snapshot, right_request) =
        consumer_fixture(0x51, "example/consumer-inspector", producer.release_id());
    let right = prepare(
        &right_snapshot,
        &right_request,
        std::slice::from_ref(&producer_bytes),
    )
    .expect("right consumer");
    assert_ne!(left.release_id(), right.release_id());
    assert!(
        test(left.bytes(), std::slice::from_ref(&producer_bytes))
            .expect("left tests")
            .all_passed()
    );
    assert!(
        test(right.bytes(), std::slice::from_ref(&producer_bytes))
            .expect("right tests")
            .all_passed()
    );

    let mut private = left_request.clone();
    private.imports[1].export = "private_canonicalize".into();
    assert_eq!(
        prepare(
            &left_snapshot,
            &private,
            std::slice::from_ref(&producer_bytes)
        )
        .expect_err("private import")
        .code,
        ErrorCode::NodeNotFound
    );

    let mut wrong_signature = left_request;
    wrong_signature.imports[1].export = "inspect_length".into();
    assert_eq!(
        prepare(
            &left_snapshot,
            &wrong_signature,
            std::slice::from_ref(&producer_bytes)
        )
        .expect_err("same-kind proxy signature mismatch")
        .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn exact_versions_coexist_and_diamond_deduplicates_shared_nominality() {
    let (r1_snapshot, r1_request) = producer_fixture(0x61, 0, false, false);
    let r1 = prepare(&r1_snapshot, &r1_request, &[]).expect("R1");
    let (r2_snapshot, r2_request) = producer_fixture(0x62, 0, false, true);
    let r2 = prepare(&r2_snapshot, &r2_request, &[]).expect("R2");
    assert_ne!(r1.release_id(), r2.release_id());
    assert_eq!(
        inspect(r1.bytes()).expect("R1 inspect").coordinate,
        inspect(r2.bytes()).expect("R2 inspect").coordinate
    );

    let (coexist_snapshot, coexist_request) = pair_consumer_fixture(
        0x63,
        "example/release-version-coexistence",
        r1.release_id(),
        r2.release_id(),
        "normalize",
        "normalize",
        b"abcabc",
    );
    let coexist_dependencies = vec![r1.bytes().to_vec(), r2.bytes().to_vec()];
    let coexist = prepare(&coexist_snapshot, &coexist_request, &coexist_dependencies)
        .expect("coexistence release");
    let coexist_decoded = decode(coexist.bytes()).expect("coexist decode");
    let coexist_graph = graph::ReleaseGraph::new(
        coexist_decoded,
        coexist_dependencies
            .iter()
            .map(|bytes| decode(bytes))
            .collect::<Result<Vec<_>>>()
            .expect("coexist dependencies"),
    )
    .expect("coexist graph");
    let flattened = coexist_graph.flatten().expect("coexist flatten");
    let r1_decoded = decode(r1.bytes()).expect("R1 decode");
    let r2_decoded = decode(r2.bytes()).expect("R2 decode");
    assert_ne!(
        flattened
            .item(r1.release_id(), export(&r1_decoded, "frame"))
            .expect("R1 frame"),
        flattened
            .item(r2.release_id(), export(&r2_decoded, "frame"))
            .expect("R2 frame")
    );

    let r1_bytes = r1.bytes().to_vec();
    let (left_snapshot, left_request) =
        consumer_fixture(0x71, "example/diamond-left", r1.release_id());
    let left = prepare(
        &left_snapshot,
        &left_request,
        std::slice::from_ref(&r1_bytes),
    )
    .expect("diamond left");
    let (right_snapshot, right_request) =
        consumer_fixture(0x72, "example/diamond-right", r1.release_id());
    let right = prepare(
        &right_snapshot,
        &right_request,
        std::slice::from_ref(&r1_bytes),
    )
    .expect("diamond right");
    let (diamond_snapshot, diamond_request) = pair_consumer_fixture(
        0x73,
        "example/release-diamond",
        left.release_id(),
        right.release_id(),
        "entry",
        "entry",
        b"abc",
    );
    let diamond_dependencies = vec![left.bytes().to_vec(), right.bytes().to_vec(), r1_bytes];
    let diamond =
        prepare(&diamond_snapshot, &diamond_request, &diamond_dependencies).expect("diamond root");
    let mut permuted_request = diamond_request.clone();
    permuted_request.dependencies.reverse();
    permuted_request.imports.reverse();
    permuted_request.exports.reverse();
    permuted_request.tests.reverse();
    let mut permuted_dependencies = diamond_dependencies.clone();
    permuted_dependencies.reverse();
    let permuted = prepare(&diamond_snapshot, &permuted_request, &permuted_dependencies)
        .expect("permuted diamond");
    assert_eq!(diamond.bytes(), permuted.bytes());
    assert_eq!(diamond.release_id(), permuted.release_id());
    let graph = graph::ReleaseGraph::new(
        decode(diamond.bytes()).expect("diamond decode"),
        diamond_dependencies
            .iter()
            .map(|bytes| decode(bytes))
            .collect::<Result<Vec<_>>>()
            .expect("diamond dependencies"),
    )
    .expect("diamond graph");
    assert_eq!(graph.releases().count(), 4);
    assert_eq!(graph.edge_count(), 4);
    assert_eq!(graph.depth(), 3);
    assert!(graph.aggregate_bytes() > diamond.bytes().len());
    assert!(
        graph
            .run_release_tests(diamond.release_id())
            .expect("diamond tests")
            .all_passed()
    );
}
