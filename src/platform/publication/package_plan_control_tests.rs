//! Operation controls must reach native package planning after host source admission.

use super::*;
use crate::platform::change::{AuthoredChange, ChangeBudget};
use crate::platform::kernel::{Name, PackageId};
use crate::platform::persistent_map::{MapContentDigest, PageDigest};
use crate::platform::publication::ExportedPackageTransport;
use crate::platform::semantic_id::RepositoryId;

fn empty_snapshot(seed: &[u8]) -> KernelSnapshot {
    let empty = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(seed, 0),
            package_id: PackageId::migrate(seed, 0),
            package_name: Name::new("native_plan_control").unwrap(),
            owners: empty,
            dependencies: empty,
            retirements: empty,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        representation_dependencies: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    }
}

fn create(root: &Path, name: &str) -> CreatedRepository {
    GraphRepository::create(&root.join(name), &empty_snapshot(name.as_bytes()), None).unwrap()
}

fn dependency_request(
    created: &CreatedRepository,
    dependency: &ExportedPackageTransport,
) -> AuthoredChangeSet {
    AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::AddDependency {
            package: dependency.revision.package,
            semantic_revision: dependency.revision.revision.revision_id().unwrap(),
            package_revision: dependency.revision_digest,
        }],
    }
}

fn source_with_dependency(
    root: &Path,
) -> (
    GraphRepository,
    ExportedPackageTransport,
    ExportedPackageTransport,
) {
    let leaf = create(root, "control-leaf")
        .repository
        .export_package_transport()
        .unwrap();
    let source = create(root, "control-source");
    source
        .repository
        .stage_package_transport(leaf.transport_digest, &leaf.container)
        .unwrap();
    let prepared = source
        .repository
        .prepare_authored_change(
            &dependency_request(&source, &leaf),
            PublicationOptions::default(),
        )
        .unwrap();
    source.repository.publish(&prepared.publication).unwrap();
    let exported = source.repository.export_package_transport().unwrap();
    (source.repository, leaf, exported)
}

fn assert_native_input_cancellation(control: &ExecutionControl, error: &Diagnostic) {
    assert!(control.is_cancelled());
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(error.code, "execution_cancelled");
    assert!(error.message.contains("invocation input"));
}

#[test]
fn controlled_package_stage_cancels_inside_native_input_without_exposing_readiness() {
    let temporary = tempfile::tempdir().unwrap();
    let (_source, leaf, exported) = source_with_dependency(temporary.path());
    let target = create(temporary.path(), "control-stage-target");
    target
        .repository
        .stage_package_transport(leaf.transport_digest, &leaf.container)
        .unwrap();
    let head_bytes = fs::read(target.repository.root().join("HEAD")).unwrap();
    let ready_path = target.repository.root().join("PACKAGE-TRANSPORTS/CURRENT");
    let ready_bytes = fs::read(&ready_path).unwrap();

    // Only VM invocation-input admission consumes this phase probe. Host catalog,
    // source validation and native artifact preparation cannot pre-cancel it.
    let control = ExecutionControl::cancel_after_input_nodes(2);
    assert!(!control.is_cancelled());
    let error = target
        .repository
        .stage_package_transport_with_control(
            exported.transport_digest,
            &exported.container,
            &control,
        )
        .unwrap_err();
    assert_native_input_cancellation(&control, &error);
    assert_eq!(fs::read(&ready_path).unwrap(), ready_bytes);
    assert_eq!(
        fs::read(target.repository.root().join("HEAD")).unwrap(),
        head_bytes
    );
    assert_eq!(
        target.repository.current().unwrap().head,
        target.current.head
    );
    assert!(
        target
            .repository
            .staged_package_source(exported.revision_digest)
            .is_err()
    );
    assert!(
        target
            .repository
            .object_store()
            .unwrap()
            .staging_leftovers()
            .is_empty()
    );

    let receipt = target
        .repository
        .stage_package_transport_with_control(
            exported.transport_digest,
            &exported.container,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(receipt.outcome, StageOutcome::Inserted);
    assert_eq!(receipt.closure_packages, 2);
    assert_eq!(receipt.closure_edges, 1);
    assert!(
        receipt.validation_visits
            <= crate::platform::package_transport::source::MAXIMUM_VALIDATION_VISITS
    );
    assert!(
        receipt.validation_read_bytes
            <= crate::platform::package_transport::source::MAXIMUM_VALIDATION_READ_BYTES
    );
    let actual = target
        .repository
        .staged_package_source(exported.revision_digest)
        .unwrap();
    let expected = crate::platform::package_transport::source::PackageContainer::decode(
        &exported.container,
        exported.transport_digest,
    )
    .unwrap();
    assert_eq!(actual.container.objects, expected.objects);
    assert_eq!(
        target.repository.current().unwrap().head,
        target.current.head
    );
}

#[test]
fn revision_pinned_export_passes_its_control_into_native_planning_and_recovers() {
    let temporary = tempfile::tempdir().unwrap();
    let (source, _leaf, expected) = source_with_dependency(temporary.path());
    let before = source.current().unwrap().head;
    let ready_path = source.root().join("PACKAGE-TRANSPORTS/CURRENT");
    let ready_bytes = fs::read(&ready_path).unwrap();
    // Pin the view before arming the export-specific input admission probe.
    let view = source.view_current().unwrap();
    let control = ExecutionControl::cancel_after_input_nodes(2);
    let view = view.with_control(&control);
    assert!(!control.is_cancelled());
    let error = view.export_package_transport().unwrap_err();
    assert_native_input_cancellation(&control, &error);
    drop(view);
    assert_eq!(source.current().unwrap().head, before);
    assert_eq!(fs::read(&ready_path).unwrap(), ready_bytes);

    let recovered = source
        .view_current_with_control(&ExecutionControl::uncancelled())
        .unwrap()
        .export_package_transport()
        .unwrap();
    assert_eq!(recovered.revision_digest, expected.revision_digest);
    assert_eq!(recovered.transport_digest, expected.transport_digest);
    assert_eq!(recovered.container, expected.container);
}

#[test]
fn prepared_dependency_change_passes_its_control_into_native_planning_and_recovers() {
    let temporary = tempfile::tempdir().unwrap();
    let (_source, _leaf, dependency) = source_with_dependency(temporary.path());
    let target = create(temporary.path(), "control-change-target");
    target
        .repository
        .stage_package_transport(dependency.transport_digest, &dependency.container)
        .unwrap();
    let request = dependency_request(&target, &dependency);
    let ready_path = target.repository.root().join("PACKAGE-TRANSPORTS/CURRENT");
    let ready_bytes = fs::read(&ready_path).unwrap();
    let control = ExecutionControl::cancel_after_input_nodes(2);
    let view = target
        .repository
        .view_current_with_control(&control)
        .unwrap();
    assert!(!control.is_cancelled());
    let errors = view
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_native_input_cancellation(&control, &errors[0]);
    drop(view);
    assert_eq!(
        target.repository.current().unwrap().head,
        target.current.head
    );
    assert_eq!(fs::read(&ready_path).unwrap(), ready_bytes);

    let prepared = target
        .repository
        .view_current_with_control(&ExecutionControl::uncancelled())
        .unwrap()
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    assert!(matches!(
        target.repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    let exported = target.repository.export_package_container().unwrap();
    assert_eq!(exported.packages.len(), 3);
    assert_eq!(exported.dependency_edges, 2);
}
