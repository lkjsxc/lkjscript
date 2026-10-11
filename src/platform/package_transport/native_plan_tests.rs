#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
use crate::platform::kernel::{
    DependencyRecord, PackageId, PackageInterfaceDigest, SemanticStateDigest,
};
use crate::platform::package_transport::{PACKAGE_REVISION_CONTRACT_VERSION, PackageRevision};
use crate::platform::publication::RevisionCore;
use crate::platform::semantic_id::RepositoryId;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

#[allow(dead_code)]
#[path = "../../../tests/native_components_support/cases.rs"]
mod capacity_cases;

fn identity(index: usize) -> PackageRevisionDigest {
    let mut bytes = [0x51; 32];
    bytes[24..].copy_from_slice(&(index as u64 + 1).to_be_bytes());
    PackageRevisionDigest::from_bytes(bytes)
}

// Fixed decoded metadata isolates topology and exact origin mapping. These are
// not fabricated hash-bound public package revisions; public admission tests own
// canonical storage, interface/body validity and binding checks independently.
fn collected(dependencies: &[Vec<usize>]) -> CollectedPackageRevisionInputs {
    let packages: Vec<_> = (0..dependencies.len())
        .map(|index| {
            let mut bytes = [0x31; 16];
            bytes[8..].copy_from_slice(&(index as u64 + 1).to_be_bytes());
            PackageId::from_bytes(bytes).unwrap()
        })
        .collect();
    let cores: Vec<_> = (0..dependencies.len())
        .map(|index| RevisionCore {
            contract_version: crate::platform::publication::contract::REVISION_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"native-package-plan-test", index as u64),
            parents: Vec::new(),
            semantic_state: SemanticStateDigest::from_bytes([0x41; 32]),
        })
        .collect();
    let revisions = dependencies
        .iter()
        .enumerate()
        .map(|(index, dependencies)| {
            let revision = PackageRevision {
                contract_version: PACKAGE_REVISION_CONTRACT_VERSION,
                graph_contract_version: GRAPH_CONTRACT_VERSION,
                package: packages[index],
                revision: cores[index].clone(),
                interface: PackageInterfaceDigest::from_bytes([0x61; 32]),
                dependencies: dependencies
                    .iter()
                    .map(|target| DependencyRecord {
                        graph_contract_version: GRAPH_CONTRACT_VERSION,
                        package: packages[*target],
                        semantic_revision: cores[*target].revision_id().unwrap(),
                        package_revision: identity(*target),
                    })
                    .collect(),
            };
            (identity(index), revision)
        })
        .collect::<BTreeMap<_, _>>();
    CollectedPackageRevisionInputs {
        root_revision: revisions[&identity(0)].clone(),
        revisions,
        dependency_edges: dependencies.iter().map(Vec::len).sum(),
    }
}

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn two_node_result() -> Value {
    json!({"case":"valid","value":{
        "components":[
            {"members":[0],"cyclic":false,"dependencies":[1]},
            {"members":[1],"cyclic":false,"dependencies":[]}],
        "stages":[[1],[0]],"reachable":[0,1],"unreachable":[]}})
}

#[test]
fn projection_preserves_complete_exact_origins_and_refuses_incomplete_inventory() {
    let control = ExecutionControl::uncancelled();
    let inputs = collected(&[vec![2, 1], vec![], vec![1]]);
    let projected = project(&inputs, identity(0), &control).unwrap();
    assert_eq!(
        projected.identities,
        vec![identity(0), identity(1), identity(2)]
    );
    assert_eq!(
        decode_strict(&projected.arguments, JSON_LIMITS).unwrap(),
        json!([{"roots":[0],"nodes":[
            {"id":0,"successors":[2,1]},
            {"id":1,"successors":[]},
            {"id":2,"successors":[1]}]}])
    );
    let mut wrong = collected(&[vec![1], vec![]]);
    wrong.dependency_edges = 0;
    assert_eq!(
        project(&wrong, identity(0), &control).err().unwrap().class,
        DiagnosticClass::Infrastructure
    );
    wrong.dependency_edges = 1;
    wrong.revisions.remove(&identity(1));
    assert_eq!(
        project(&wrong, identity(0), &control).err().unwrap().code,
        "package_native_plan_edge"
    );
    assert_eq!(
        project(&inputs, identity(3), &control).err().unwrap().class,
        DiagnosticClass::Infrastructure
    );
}

#[test]
fn complete_output_mapping_refuses_foreign_partial_duplicate_and_negative_reports() {
    let control = ExecutionControl::uncancelled();
    let identities = [identity(0), identity(1)];
    assert_eq!(
        admit_output(&bytes(&two_node_result()), &identities, &control).unwrap(),
        vec![identity(1), identity(0)]
    );
    for outcome in [
        json!({"case":"invalid","value":{"code":"missing-root","owner":-1,"target":0}}),
        json!({"case":"capacity","value":"nodes"}),
        json!({"case":"inconsistent","value":"source-outcome"}),
    ] {
        assert_eq!(
            admit_output(&bytes(&outcome), &identities, &control)
                .unwrap_err()
                .class,
            DiagnosticClass::Infrastructure
        );
    }
    let baseline = two_node_result();
    for (pointer, value) in [
        ("/value/reachable", json!([0])),
        ("/value/unreachable", json!([1])),
        ("/value/components/0/members", json!([0, 0])),
        ("/value/components/1/members", json!([])),
        ("/value/components/1/members", json!([2])),
        ("/value/components/0/dependencies", json!([1, 1])),
        ("/value/stages", json!([[1]])),
        ("/value/stages", json!([[], [1], [0]])),
        ("/value/stages", json!([[1], [0], [0]])),
        ("/value/stages", json!([[-1], [0]])),
        ("/value/stages", json!([[1], [2]])),
    ] {
        let mut changed = baseline.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert_eq!(
            admit_output(&bytes(&changed), &identities, &control)
                .unwrap_err()
                .class,
            DiagnosticClass::Infrastructure,
            "{pointer}: {changed}"
        );
    }
    let mut extra = baseline;
    extra["value"]["extra"] = json!(true);
    assert_eq!(
        admit_output(&bytes(&extra), &identities, &control)
            .unwrap_err()
            .code,
        "package_native_plan_output"
    );
    for raw in [
        b"{\"case\":\"valid\",\"case\":\"capacity\",\"value\":\"nodes\"}".as_slice(),
        b"{\"case\":\"capacity\",\"value\":\"nodes\"} null".as_slice(),
    ] {
        assert_eq!(
            admit_output(raw, &identities, &control).unwrap_err().class,
            DiagnosticClass::Infrastructure
        );
    }
}

#[test]
fn checked_cycle_mapping_requires_a_complete_result_and_retains_semantic_failure() {
    let control = ExecutionControl::uncancelled();
    let identities = [identity(0), identity(1)];
    let cycle = json!({"case":"valid","value":{
        "components":[{"members":[0,1],"cyclic":true,"dependencies":[]}],
        "stages":[[0]],"reachable":[0,1],"unreachable":[]}});
    let error = admit_output(&bytes(&cycle), &identities, &control).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "package_revision_dependency_cycle");
    let mut missing = cycle;
    missing["value"]["stages"] = json!([]);
    assert_eq!(
        admit_output(&bytes(&missing), &identities, &control)
            .unwrap_err()
            .class,
        DiagnosticClass::Infrastructure
    );
}

#[test]
fn preparation_failure_and_cancellation_leave_a_retryable_unpoisoned_cache() {
    let cache = PreparationCache::<usize>::new();
    let attempts = AtomicUsize::new(0);
    let cancelled = ExecutionControl::uncancelled();
    let first = cache.get_or_prepare(&cancelled, |control| {
        attempts.fetch_add(1, Ordering::SeqCst);
        control.cancel();
        checkpoint(control)?;
        Ok(7)
    });
    assert_eq!(first.unwrap_err().class, DiagnosticClass::Cancelled);
    let healthy = ExecutionControl::uncancelled();
    let value = cache
        .get_or_prepare(&healthy, |_| {
            attempts.fetch_add(1, Ordering::SeqCst);
            Ok(17)
        })
        .unwrap();
    assert_eq!(*value, 17);
    let retained = cache
        .get_or_prepare(&healthy, |_| panic!("cached preparation repeated"))
        .unwrap();
    assert!(Arc::ptr_eq(&value, &retained));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(
        cache
            .get_or_prepare(&cancelled, |_| panic!("cancelled caller prepared"))
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );

    let failed = PreparationCache::<usize>::new();
    assert!(
        std::panic::catch_unwind(
            || failed.get_or_prepare(&healthy, |_| panic!("controlled initializer failure"))
        )
        .is_err()
    );
    assert_eq!(*failed.get_or_prepare(&healthy, |_| Ok(23)).unwrap(), 23);
}

#[test]
fn cancelled_waiter_does_not_interrupt_another_owners_preparation() {
    let cache = Arc::new(PreparationCache::<usize>::new());
    let (entered, entering) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let worker_cache = Arc::clone(&cache);
    let producer = std::thread::spawn(move || {
        worker_cache
            .get_or_prepare(&ExecutionControl::uncancelled(), |_| {
                entered.send(()).unwrap();
                released.recv().unwrap();
                Ok(31)
            })
            .unwrap()
    });
    entering.recv().unwrap();
    let waiter_control = ExecutionControl::uncancelled();
    let waiting_control = waiter_control.clone();
    let waiting_cache = Arc::clone(&cache);
    let (completed, completion) = mpsc::channel();
    let waiter = std::thread::spawn(move || {
        completed
            .send(waiting_cache.get_or_prepare(&waiting_control, |_| {
                panic!("waiter stole live preparation")
            }))
            .unwrap();
    });
    waiter_control.cancel();
    let waiting_result = completion.recv_timeout(Duration::from_secs(5));
    // Release the real initializer before asserting the observer result, ensuring
    // test failure never leaves an owned thread waiting on its release channel.
    release.send(()).unwrap();
    let prepared = producer.join().unwrap();
    waiter.join().unwrap();
    assert_eq!(
        waiting_result.unwrap().unwrap_err().class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(*prepared, 31);
    assert!(Arc::ptr_eq(
        &prepared,
        &cache
            .get_or_prepare(&ExecutionControl::uncancelled(), |_| panic!(
                "successful concurrent preparation repeated"
            ))
            .unwrap()
    ));
}

#[test]
fn bounded_encoding_reserves_before_exceeding_its_complete_byte_envelope() {
    let mut writer = BoundedJson::default();
    writer
        .write_all(&vec![b' '; JSON_LIMITS.maximum_bytes])
        .unwrap();
    assert!(writer.write_all(b"x").is_err());
    assert_eq!(writer.bytes.len(), JSON_LIMITS.maximum_bytes);
}

// Synchronous longest-path relaxation is independent of the native readiness
// queue. Test fixtures guarantee acyclicity and complete root reachability.
fn expected_order(dependencies: &[Vec<usize>]) -> Vec<PackageRevisionDigest> {
    let mut levels = vec![0_usize; dependencies.len()];
    for round in 0..=dependencies.len() {
        let next = dependencies
            .iter()
            .map(|targets| {
                targets
                    .iter()
                    .map(|target| levels[*target] + 1)
                    .max()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        if levels == next {
            let mut positions = (0..dependencies.len()).collect::<Vec<_>>();
            positions.sort_by_key(|position| (levels[*position], *position));
            return positions.into_iter().map(identity).collect();
        }
        assert!(
            round < dependencies.len(),
            "oracle fixture unexpectedly cyclic"
        );
        levels = next;
    }
    unreachable!("bounded oracle relaxation exhausted")
}

#[test]
fn successful_native_receipt_binds_exact_artifact_projection_outcome_and_consumed_order() {
    let inputs = collected(&[vec![2, 1], vec![2], vec![]]);
    let control = ExecutionControl::uncancelled();
    let projection = project(&inputs, identity(0), &control).unwrap();
    let output = invoke(&projection.arguments, &control).unwrap();
    let result = plan(&inputs, identity(0), &control).unwrap();
    let evidence = result.evidence();
    let literal_input = br#"[{"roots":[0],"nodes":[{"id":0,"successors":[2,1]},{"id":1,"successors":[2]},{"id":2,"successors":[]}]}]"#;
    assert_eq!(projection.arguments, literal_input);
    assert_eq!(
        evidence.artifact_sha256,
        encode_hex(&Sha256::digest(ARTIFACT))
    );
    assert_eq!(
        evidence.input_sha256,
        encode_hex(&Sha256::digest(literal_input))
    );
    assert_eq!(
        evidence.outcome_sha256,
        encode_hex(&Sha256::digest(&output))
    );
    let ordered_bytes: Vec<_> = [identity(2), identity(1), identity(0)]
        .into_iter()
        .flat_map(PackageRevisionDigest::bytes)
        .collect();
    assert_eq!(
        evidence.order_sha256,
        encode_hex(&Sha256::digest(&ordered_bytes))
    );
    assert_eq!((evidence.packages, evidence.edges), (3, 3));
    assert_eq!(
        result.into_order(),
        vec![identity(2), identity(1), identity(0)]
    );
}

#[test]
fn native_package_planning_consumes_independently_expected_complete_stages() {
    let control = ExecutionControl::uncancelled();
    for mask in 0..8 {
        let mut dependencies = vec![vec![1, 2, 3], vec![], vec![], vec![]];
        for (bit, (source, target)) in [(1, 2), (1, 3), (2, 3)].into_iter().enumerate() {
            if mask & (1 << bit) != 0 {
                dependencies[source].push(target);
            }
        }
        assert_eq!(
            plan(&collected(&dependencies), identity(0), &control)
                .unwrap()
                .into_order(),
            expected_order(&dependencies),
            "mask {mask}"
        );
    }
    let diamond = vec![vec![1, 2], vec![3], vec![4], vec![], vec![]];
    assert_eq!(
        plan(&collected(&diamond), identity(0), &control)
            .unwrap()
            .into_order(),
        vec![
            identity(3),
            identity(4),
            identity(1),
            identity(2),
            identity(0)
        ]
    );
    for dependencies in [vec![vec![0]], vec![vec![1], vec![0]]] {
        let error = plan(&collected(&dependencies), identity(0), &control).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Semantic);
        assert_eq!(error.code, "package_revision_dependency_cycle");
    }
    assert_eq!(
        plan(&collected(&[vec![]]), identity(0), &control)
            .unwrap()
            .into_order(),
        vec![identity(0)]
    );
}

#[test]
fn native_capacity_admission_is_complete_above_default_json_bytes_and_entries() {
    let control = ExecutionControl::uncancelled();
    for (label, proposal) in capacity_cases::capacity_suite() {
        let arguments = bytes(&json!([proposal]));
        if label == "edges-exact" {
            assert!(arguments.len() > JsonLimits::default().maximum_bytes);
            assert!(decode_strict(&arguments, JsonLimits::default()).is_err());
            let byte_only = JsonLimits {
                maximum_bytes: JSON_LIMITS.maximum_bytes,
                ..JsonLimits::default()
            };
            assert!(
                decode_strict(&arguments, byte_only).is_err(),
                "entry capacity is independent of bytes"
            );
        }
        let outcome = decode_outcome(&invoke(&arguments, &control).unwrap(), &control).unwrap();
        match (label, outcome) {
            ("nodes-exact", NativeOutcome::Valid(plan)) => {
                assert_eq!(plan.components.len(), MAXIMUM_PACKAGE_CLOSURE);
                assert!(
                    plan.components
                        .iter()
                        .enumerate()
                        .all(|(index, component)| component.members == vec![index as i64]
                            && !component.cyclic
                            && component.dependencies.is_empty())
                );
                assert_eq!(plan.stages, vec![(0..10000).collect::<Vec<i64>>()]);
                assert!(plan.reachable.is_empty());
                assert_eq!(plan.unreachable, (0..10000).collect::<Vec<i64>>());
            }
            ("roots-exact", NativeOutcome::Valid(plan)) => {
                assert_eq!(plan.components.len(), 1);
                assert_eq!(plan.components[0].members, vec![-1]);
                assert!(!plan.components[0].cyclic);
                assert!(plan.components[0].dependencies.is_empty());
                assert_eq!(plan.stages, vec![vec![-1]]);
                assert_eq!(plan.reachable, vec![-1]);
                assert!(plan.unreachable.is_empty());
            }
            ("edges-exact", NativeOutcome::Valid(plan)) => {
                assert_eq!(plan.components.len(), 1);
                assert_eq!(plan.components[0].members, vec![i64::MIN]);
                assert!(plan.components[0].cyclic);
                assert!(plan.components[0].dependencies.is_empty());
                assert_eq!(plan.stages, vec![vec![i64::MIN]]);
                assert!(plan.reachable.is_empty());
                assert_eq!(plan.unreachable, vec![i64::MIN]);
            }
            (
                "nodes-over" | "nodes-precede-roots-edges-structure",
                NativeOutcome::Capacity(dimension),
            ) => assert_eq!(dimension, "nodes", "{label}"),
            (
                "roots-over" | "roots-precede-edges-structure",
                NativeOutcome::Capacity(dimension),
            ) => assert_eq!(dimension, "roots", "{label}"),
            ("edges-over" | "edges-precede-structure", NativeOutcome::Capacity(dimension)) => {
                assert_eq!(dimension, "edges", "{label}")
            }
            (_, outcome) => panic!("{label}: unexpected complete native outcome {outcome:?}"),
        }
    }
    let malformed = bytes(&json!([{"roots":[],"nodes":[{"id":0,"successors":[false]}]}]));
    assert_eq!(
        invoke(&malformed, &control).unwrap_err().code,
        "normalized_json_type"
    );
    assert_eq!(
        plan(&collected(&[vec![]]), identity(0), &control)
            .unwrap()
            .into_order(),
        vec![identity(0)]
    );
}

#[test]
fn native_full_package_and_distinct_edge_capacities_share_one_complete_projection() {
    let mut dependencies = vec![Vec::new(); MAXIMUM_PACKAGE_CLOSURE];
    dependencies[0] = (1..MAXIMUM_PACKAGE_CLOSURE).collect();
    let mut remaining = MAXIMUM_PACKAGE_CLOSURE_EDGES - dependencies[0].len();
    for (source, targets) in dependencies.iter_mut().enumerate().skip(1) {
        let count = remaining.min(MAXIMUM_PACKAGE_CLOSURE - source - 1);
        targets.extend(source + 1..source + 1 + count);
        remaining -= count;
    }
    assert_eq!(remaining, 0);
    let inputs = collected(&dependencies);
    assert_eq!(inputs.revisions.len(), MAXIMUM_PACKAGE_CLOSURE);
    assert_eq!(inputs.dependency_edges, MAXIMUM_PACKAGE_CLOSURE_EDGES);
    let control = ExecutionControl::uncancelled();
    let projection = project(&inputs, identity(0), &control).unwrap();
    let byte_only = JsonLimits {
        maximum_bytes: JSON_LIMITS.maximum_bytes,
        ..JsonLimits::default()
    };
    assert!(decode_strict(&projection.arguments, byte_only).is_err());
    let output = invoke(&projection.arguments, &control).unwrap();
    assert!(
        output.len() > JsonLimits::default().maximum_bytes,
        "full result representation is an independent byte cost"
    );
    let order = admit_output(&output, &projection.identities, &control).unwrap();
    assert_eq!(order, expected_order(&dependencies));
}

#[test]
fn cancelled_native_invocations_do_not_publish_a_plan_or_poison_recovery() {
    let inputs = collected(&[vec![1], vec![]]);
    let before = ExecutionControl::uncancelled();
    before.cancel();
    assert_eq!(
        plan(&inputs, identity(0), &before).unwrap_err().class,
        DiagnosticClass::Cancelled
    );
    let projection = project(&inputs, identity(0), &ExecutionControl::uncancelled()).unwrap();
    assert_eq!(
        invoke(
            &projection.arguments,
            &ExecutionControl::cancel_after_input_nodes(1)
        )
        .unwrap_err()
        .class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(
        plan(&inputs, identity(0), &ExecutionControl::uncancelled())
            .unwrap()
            .into_order(),
        vec![identity(1), identity(0)]
    );
}
