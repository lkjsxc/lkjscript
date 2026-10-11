// Included in source.rs tests: real exact packages and independent compilation.

fn native_plan_empty_package(seed: &[u8]) -> AdmittedClosure {
    let directory = tempfile::tempdir().unwrap();
    GraphRepository::create(
        &directory.path().join("source"),
        &dependency_copy_empty(seed),
        None,
    )
    .unwrap()
    .repository
    .export_package_container()
    .unwrap()
}

fn native_plan_compile_in_order(
    source: &AdmittedClosure,
    order: &[PackageRevisionDigest],
) -> BTreeMap<PackageRevisionDigest, Vec<u8>> {
    use crate::platform::compiler::{LoadedArtifact, compile_immutable, load_artifact};
    let mut prepared = BTreeMap::<PackageRevisionDigest, LoadedArtifact>::new();
    let mut bytes = BTreeMap::new();
    for revision in order {
        let package = &source.packages[revision];
        let dependencies = package
            .revision
            .dependencies
            .iter()
            .map(|dependency| prepared[&dependency.package_revision].clone())
            .collect::<Vec<_>>();
        let linked = compile_immutable(package, &source.container.objects, &dependencies).unwrap();
        prepared.insert(*revision, load_artifact(&linked.artifact.bytes).unwrap());
        bytes.insert(*revision, linked.artifact.bytes);
    }
    assert_eq!(prepared.len(), source.packages.len());
    bytes
}

#[test]
fn native_package_plan_changes_stage_order_without_changing_canonical_artifacts() {
    // Pick exact independently created leaves, then a real dependent whose digest
    // precedes the higher leaf. The former digest-priority Kahn pass interleaves
    // that dependent; the checked native planner finishes the complete first stage.
    let mut leaves = (0..8)
        .map(|index| native_plan_empty_package(format!("native-plan-leaf-{index}").as_bytes()))
        .collect::<Vec<_>>();
    leaves.sort_by_key(|leaf| leaf.container.root.package_revision);
    let low = &leaves[0];
    let high = &leaves[leaves.len() - 1];
    let mut selected = None;
    for index in 0..32 {
        let dependent =
            dependency_copy_importer(format!("native-plan-dependent-{index}").as_bytes(), &[low]);
        if dependent.container.root.package_revision < high.container.root.package_revision {
            selected = Some(dependent);
            break;
        }
    }
    let dependent = selected.expect("deterministic fixture must distinguish both orders");
    let root = dependency_copy_importer(b"native-plan-order-root", &[&dependent, high]);
    let low_id = low.container.root.package_revision;
    let high_id = high.container.root.package_revision;
    let dependent_id = dependent.container.root.package_revision;
    let root_id = root.container.root.package_revision;
    assert!(low_id < high_id && dependent_id < high_id);
    let old_order = [low_id, dependent_id, high_id, root_id];
    assert_eq!(
        root.dependency_order,
        [low_id, high_id, dependent_id, root_id]
    );
    assert_ne!(root.dependency_order, old_order);
    let exact_source = root.container.encode().unwrap();
    let native = native_plan_compile_in_order(&root, &root.dependency_order);
    let predecessor_order = native_plan_compile_in_order(&root, &old_order);
    assert_eq!(native, predecessor_order);
    assert_eq!(root.container.encode().unwrap(), exact_source);

    let decoded = PackageContainer::decode(&exact_source, root.container.root.transport).unwrap();
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    assert_eq!(
        decoded.admit_with_control(&cancelled).unwrap_err().code,
        "execution_cancelled"
    );
    let recovered = decoded
        .admit_with_control(&ExecutionControl::uncancelled())
        .unwrap();
    assert_eq!(recovered.dependency_order, root.dependency_order);
    assert_eq!(recovered.container.encode().unwrap(), exact_source);
}
