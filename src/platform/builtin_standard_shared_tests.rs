//! Shared builtin proofs must preserve exact source, independent expectations and publication.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::platform::execution::ExecutionControl;
use crate::platform::package_transport::{oracle, source};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};

#[test]
fn shared_builtin_retention_accounts_for_aggregate_copies_and_program_reservation() {
    let standard = BuiltinStandard::load().unwrap();
    let material = standard.retained_material;
    assert!(material.bytes > standard.transport_bytes.len() + standard.artifact_bytes.len());
    assert!(material.bytes > retention::PROGRAM_MATERIAL_RESERVATION);
    assert!(material.records > standard.admitted_source.container.objects.len());
    material
        .require_within(material.bytes, material.records)
        .unwrap();
    assert!(
        material
            .require_within(material.bytes - 1, material.records)
            .is_err()
    );
    assert!(
        material
            .require_within(material.bytes, material.records - 1)
            .is_err()
    );
    let program = standard.prepared_program().unwrap();
    let program_material = retention::program_material(standard.artifact_bytes.len(), &program)
        .unwrap()
        .require_within(
            retention::PROGRAM_MATERIAL_RESERVATION,
            MAXIMUM_RETAINED_BUILTIN_OBJECTS - material.records,
        )
        .unwrap();
    assert!(program_material.bytes > standard.artifact_bytes.len());
    eprintln!(
        "builtin modeled retained material: {} bytes, {} records; reserved program {} bytes, actual program model {} bytes, {} records",
        material.bytes,
        material.records,
        retention::PROGRAM_MATERIAL_RESERVATION,
        program_material.bytes,
        program_material.records,
    );
}

#[test]
fn shared_builtin_program_cancellation_does_not_poison_future_callers() {
    let standard = BuiltinStandard::load().unwrap();
    let mut isolated = standard.clone();
    isolated.program = Arc::new(Mutex::new(None));
    let cancelled = ExecutionControl::cancel_after_checks(2);
    assert_eq!(
        isolated
            .prepared_program_with_control(&cancelled)
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled,
    );
    assert!(isolated.program.lock().unwrap().is_none());
    let program = isolated.prepared_program().unwrap();
    assert!(Arc::ptr_eq(&program, &isolated.prepared_program().unwrap()));

    // The production hook accepts only a fresh strict admission of these same bytes.
    let artifact = load_artifact(standard.artifact_bytes()).unwrap();
    let shared = prepare_admitted_program(
        standard.artifact_bytes(),
        artifact,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert!(Arc::ptr_eq(&shared, &standard.prepared_program().unwrap()));
    let mut corrupt = standard.artifact_bytes().to_vec();
    corrupt.push(0xff);
    assert!(
        crate::platform::native_tool::PureTool::load(&corrupt, &ExecutionControl::uncancelled(),)
            .is_err()
    );
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    assert_eq!(
        isolated
            .prepared_program_with_control(&cancelled)
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled,
    );
    assert!(Arc::ptr_eq(&program, &isolated.prepared_program().unwrap()));
}

#[test]
fn shared_builtin_source_and_program_reuse_complete_process_proofs() {
    let standard = BuiltinStandard::load().unwrap();
    let original = standard.admitted_source();
    let revision = standard.package_revision;
    let before = crate::platform::witness::checked_full_validation_invocations();
    let cold = without_shared_source(|| original.container.admit()).unwrap();
    assert_eq!(
        crate::platform::witness::checked_full_validation_invocations() - before,
        1
    );
    let before = crate::platform::witness::checked_full_validation_invocations();
    let warm = original.container.admit().unwrap();
    assert_eq!(
        crate::platform::witness::checked_full_validation_invocations(),
        before
    );
    assert!(Arc::ptr_eq(
        &original.packages[&revision],
        &warm.packages[&revision]
    ));
    assert!(!Arc::ptr_eq(
        &cold.packages[&revision],
        &warm.packages[&revision]
    ));
    assert_eq!(
        cold.container.encode().unwrap(),
        warm.container.encode().unwrap()
    );
    assert_eq!(
        cold.packages[&revision].witness.manifest_bytes,
        warm.packages[&revision].witness.manifest_bytes
    );
    assert_eq!(
        cold.packages[&revision].witness.report,
        warm.packages[&revision].witness.report
    );
    let before = oracle::full_package_validations();
    let independent = without_shared_source(|| oracle::reconstruct(&warm.container)).unwrap();
    assert_eq!(oracle::full_package_validations() - before, 1);
    let before = oracle::full_package_validations();
    let reused = oracle::reconstruct(&warm.container).unwrap();
    assert_eq!(oracle::full_package_validations(), before);
    assert_eq!(
        independent.snapshots[&standard.package].owners,
        reused.snapshots[&standard.package].owners
    );
    assert_eq!(
        independent.snapshots[&standard.package].types,
        reused.snapshots[&standard.package].types
    );
    assert_eq!(independent.interfaces, reused.interfaces);
    assert_eq!(
        independent.snapshots[&standard.package].owners,
        warm.packages[&revision].snapshot.owners
    );
    let first = standard.prepared_program().unwrap();
    let second = standard.prepared_program().unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(first.root_package, standard.package);
    assert_eq!(first.root_revision, standard.semantic_revision);
    assert!(standard.matches_source(
        original.container.root,
        crate::platform::canonical_schema::SCHEMA_EPOCH,
        crate::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH
    ));
    assert!(!standard.matches_source(
        original.container.root,
        crate::platform::canonical_schema::SCHEMA_EPOCH + 1,
        crate::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH
    ));
    let mut foreign = crate::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH;
    foreign[0] ^= 1;
    assert!(!standard.matches_source(
        original.container.root,
        crate::platform::canonical_schema::SCHEMA_EPOCH,
        foreign
    ));
}

#[test]
fn shared_builtin_source_still_admits_private_bytes_limits_and_cancellation() {
    let standard = BuiltinStandard::load().unwrap();
    let original = standard.admitted_source();
    let package = &original.packages[&standard.package_revision];
    let owner = package
        .snapshot
        .owners
        .values()
        .find(|owner| matches!(owner, OwnerRecord::Expression(_)))
        .unwrap();
    let digest = crate::platform::kernel::encode_owner(owner).unwrap().0;
    let key = ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes());
    let mut missing = original.container.clone();
    missing.objects.remove(&key).unwrap();
    assert!(missing.admit().is_err());
    assert!(oracle::reconstruct(&missing).is_err());
    let mut corrupt = original.container.clone();
    corrupt.objects.get_mut(&key).unwrap().push(0xff);
    assert!(corrupt.admit().is_err());
    assert!(oracle::reconstruct(&corrupt).is_err());
    let warm = original.container.admit().unwrap();
    let exact = original
        .container
        .admit_with_budget(warm.validation_visits, warm.validation_read_bytes)
        .unwrap();
    assert_eq!(exact.validation_visits, warm.validation_visits);
    assert_eq!(exact.validation_read_bytes, warm.validation_read_bytes);
    assert!(
        original
            .container
            .admit_with_budget(warm.validation_visits - 1, warm.validation_read_bytes)
            .is_err()
    );
    assert!(
        original
            .container
            .admit_with_budget(warm.validation_visits, warm.validation_read_bytes - 1)
            .is_err()
    );
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    assert_eq!(
        original
            .container
            .admit_with_control(&cancelled)
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );
    assert!(Arc::ptr_eq(
        &original.packages[&standard.package_revision],
        &original.container.admit().unwrap().packages[&standard.package_revision]
    ));
}

#[test]
fn shared_builtin_source_never_substitutes_another_exact_transport() {
    let standard = BuiltinStandard::load().unwrap();
    let source = standard.admitted_source();
    let package = &source.packages[&standard.package_revision];
    let alternate = crate::platform::package_interface::build_package_interface_with_leaf_target(
        &package.interface_owners,
        &package.interface_types,
        1,
    )
    .unwrap();
    assert_eq!(
        alternate.root.content_root(),
        package.transport.interface_owners.content_root()
    );
    assert_ne!(
        alternate.root.page(),
        package.transport.interface_owners.page()
    );
    let mut transport = package.transport.clone();
    transport.interface_owners = alternate.root;
    let (digest, bytes) = transport.encode().unwrap();
    let binding = PackageTransportBinding {
        package_revision: standard.package_revision,
        transport: digest,
    };
    assert!(BuiltinStandard::initialized_for(binding).is_none());
    let mut objects = source.container.objects.clone();
    objects.extend(alternate.objects);
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageTransport, digest.bytes()),
        bytes,
    );
    let before = crate::platform::witness::checked_full_validation_invocations();
    let admitted = source::collect(&objects, binding, &[binding]).unwrap();
    assert_eq!(
        crate::platform::witness::checked_full_validation_invocations() - before,
        1
    );
    assert_eq!(admitted.container.root, binding);
    assert!(!Arc::ptr_eq(
        package,
        &admitted.packages[&standard.package_revision]
    ));
    let independent = oracle::reconstruct(&admitted.container).unwrap();
    assert_eq!(
        independent.snapshots[&standard.package].owners,
        package.snapshot.owners
    );
}

#[test]
fn shared_builtin_source_cold_and_warm_publications_keep_identical_canonical_receipts() {
    use crate::platform::publication::{GraphRepository, PublicationOutcome};
    let standard = BuiltinStandard::load().unwrap();
    let memory = crate::platform::MemProjectStore::new(256 * 1_048_576).unwrap();
    crate::platform::create_memory_project(&memory, "shared-proof", "minimal").unwrap();
    let repository = GraphRepository::open_memory(&memory).unwrap();
    repository
        .stage_package_transport(standard.package_transport, standard.transport_bytes())
        .unwrap();
    let fork = memory.fork().unwrap();
    let other = GraphRepository::open_memory(&fork).unwrap();
    let base = repository.view_current().unwrap().revision();
    let request = format!(
        "request base={base} idempotency=shared-builtin\nadd.dependency package={} semantic-revision={} package-revision={}\ncreate.module as=$module name=shared-proof\n",
        standard.package, standard.semantic_revision, standard.package_revision,
    );
    let decoded =
        crate::platform::control::decode_compact_change("shared-source", request.as_bytes())
            .unwrap();
    let cold = without_shared_source(|| {
        repository.prepare_authored_change(&decoded.semantic, decoded.options.clone())
    })
    .unwrap();
    let warm = other
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert_eq!(cold.publication.head_bytes, warm.publication.head_bytes);
    assert_eq!(
        cold.publication.receipt_bytes,
        warm.publication.receipt_bytes
    );
    assert_eq!(
        cold.publication.transaction_bytes,
        warm.publication.transaction_bytes
    );
    assert_eq!(
        cold.publication.semantic_diff_bytes,
        warm.publication.semantic_diff_bytes
    );
    assert_eq!(cold.publication.objects, warm.publication.objects);
    assert!(matches!(
        without_shared_source(|| repository.publish(&cold.publication)).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert!(matches!(
        other.publish(&warm.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert_eq!(
        repository.current().unwrap().head,
        other.current().unwrap().head
    );
}
