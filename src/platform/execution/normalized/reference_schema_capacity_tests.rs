//! Raw synthetic snapshots isolate the canonical inventory/topology layer.
//! They contain no host-normalized IR or reusable admission certificates.

use super::*;
use crate::platform::execution::{ExecutionControl, ExecutionFailureClass};
use crate::platform::kernel::{DependencyRecord, PackageRevisionDigest, SemanticRoot};
use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use crate::platform::semantic_id::{RepositoryId, RevisionId};

const SEED: &[u8] = b"reference-capacity-independent-inventories";

fn empty(ordinal: u64) -> Result<KernelSnapshot, ExecutionError> {
    let map = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    Ok(KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(SEED, ordinal),
            package_id: PackageId::migrate(SEED, ordinal),
            package_name: Name::new("raw-reference-capacity").map_err(source_error)?,
            owners: map,
            dependencies: map,
            retirements: map,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        representation_dependencies: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    })
}

fn dependency(package: PackageId) -> DependencyRecord {
    let mut identity = [0; 32];
    identity[..16].copy_from_slice(&package.bytes());
    identity[16..].copy_from_slice(&package.bytes());
    DependencyRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        package,
        semantic_revision: RevisionId::from_digest(identity),
        package_revision: PackageRevisionDigest::from_bytes(identity),
    }
}

#[test]
fn reference_capacity_schema_reconstructs_more_than_ten_thousand_packages()
-> Result<(), ExecutionError> {
    let mut inputs = source_items(10_001)?;
    for ordinal in 0..10_001 {
        inputs.push(empty(ordinal)?);
    }
    let schema = NormalizedReferenceSchema::reconstruct(inputs.iter())?;
    assert!(schema.functions.is_empty() && schema.types.is_empty());
    assert!(schema.source_admission_steps > 0);
    assert!(!schema.source_admission_saturated);

    // The removed count ceiling must not weaken namespace uniqueness.
    let duplicate = NormalizedReferenceSchema::reconstruct([&inputs[0], &inputs[0]]);
    assert!(
        duplicate
            .as_ref()
            .is_err_and(|error| error.code == "normalized_reference_inventory"),
        "{duplicate:?}"
    );
    Ok(())
}

#[test]
fn reference_capacity_dependency_topology_crosses_one_hundred_thousand_edges()
-> Result<(), ExecutionError> {
    // 313 independent consumers each select 320 suppliers: 100,160 edges.
    // This control targets the topological reader; correspondence has its own
    // complete interface/body tests and is not bypassed in reconstruction.
    let mut snapshots = source_items(633)?;
    for ordinal in 0..633 {
        snapshots.push(empty(ordinal)?);
    }
    let suppliers = snapshots[..320]
        .iter()
        .map(|source| source.root.package_id)
        .collect::<Vec<_>>();
    for consumer in &mut snapshots[320..] {
        for package in &suppliers {
            consumer.dependencies.insert(*package, dependency(*package));
        }
    }
    let inputs = snapshots.iter().collect::<Vec<_>>();
    let mut work = Meter::new(None);
    validate_dependency_closure(&inputs, &ExecutionControl::uncancelled(), &mut work)?;
    assert_eq!(work.observation(), Observation::exact(2 * (633 + 100_160)));

    let last = snapshots[632].root.package_id;
    snapshots[0].dependencies.insert(last, dependency(last));
    let inputs = snapshots.iter().collect::<Vec<_>>();
    let cycle = validate_dependency_closure(
        &inputs,
        &ExecutionControl::uncancelled(),
        &mut Meter::new(None),
    );
    assert!(
        cycle
            .as_ref()
            .is_err_and(|error| error.code == "normalized_reference_inventory"
                && error.message.contains("cyclic")),
        "{cycle:?}"
    );
    Ok(())
}

#[test]
fn reference_capacity_dependency_topology_keeps_finite_policy_and_saturation_distinct()
-> Result<(), ExecutionError> {
    let left = empty(0)?;
    let mut right = empty(1)?;
    right
        .dependencies
        .insert(left.root.package_id, dependency(left.root.package_id));
    let inputs = [&left, &right];
    let control = ExecutionControl::uncancelled();
    let mut measured = Meter::new(None);
    validate_dependency_closure(&inputs, &control, &mut measured)?;
    assert!(!measured.observation().saturated);
    let exact = measured.observation().consumed;
    for (limit, accepts) in [
        (0, false),
        (exact - 1, false),
        (exact, true),
        (u64::MAX, true),
    ] {
        let mut work = Meter::new(Some(limit));
        let result = validate_dependency_closure(&inputs, &control, &mut work);
        if accepts {
            result?;
            assert_eq!(work.remaining(), Some(limit - exact));
        } else {
            assert!(
                result
                    .as_ref()
                    .is_err_and(|error| error.class == ExecutionFailureClass::Resource),
                "{result:?}"
            );
        }
    }
    let mut saturated = Meter::new(None);
    assert!(saturated.charge(Observation::exact(u64::MAX)).is_ok());
    validate_dependency_closure(&inputs, &control, &mut saturated)?;
    assert!(saturated.observation().saturated);
    let mut missing = right.clone();
    let absent = PackageId::migrate(SEED, 100);
    missing.dependencies.insert(absent, dependency(absent));
    let rejection = validate_dependency_closure(&[&left, &missing], &control, &mut saturated);
    assert!(
        rejection
            .as_ref()
            .is_err_and(|error| error.code == "normalized_reference_inventory"),
        "{rejection:?}"
    );

    let cancellation = validate_dependency_closure(
        &inputs,
        &ExecutionControl::cancel_after_checks(0),
        &mut Meter::new(None),
    );
    assert!(
        cancellation
            .as_ref()
            .is_err_and(|error| error.class == ExecutionFailureClass::Cancelled),
        "{cancellation:?}"
    );
    Ok(())
}
