// Included by source.rs. Real canonical reconstruction and independent source oracles
// exercise the optional, invocation-bound complete full-validation proof reuse.

pub(super) fn source_validation_reuse_fixture() -> AdmittedClosure {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create repeated
  (function create choose (visibility public) (effect pure)
    (parameter create flag (type Bool)) (returns I64)
    (body (if (local flag) (i64 17) (i64 23))))
  (function create private-choice (visibility private) (effect pure)
    (parameter create flag (type Bool)) (returns I64)
    (body (if (local flag) (i64 29) (i64 31))))
  (function create private-text (visibility private) (effect pure)
    (returns Text) (body (text "complete private blob")))))
declarations.end"#,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    GraphRepository::create(&directory.path().join("source"), &snapshot, None)
        .unwrap()
        .repository
        .export_package_container()
        .unwrap()
}

pub(super) fn source_validation_reuse_plan(
    container: &PackageContainer,
) -> super::super::PackageTransportClosureValidation {
    super::super::validate_package_transport_closure(
        &container.objects,
        container.root.package_revision,
        &container.selections,
        None,
        &mut StoreWork::default(),
    )
    .unwrap()
}

fn source_validation_reuse_alternate_transport(source: &AdmittedClosure) -> PackageContainer {
    let package = &source.packages[&source.container.root.package_revision];
    assert!(package.revision.dependencies.is_empty());
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
        package_revision: source.container.root.package_revision,
        transport: digest,
    };
    let mut objects = source.container.objects.clone();
    objects.extend(alternate.objects);
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageTransport, digest.bytes()),
        bytes,
    );
    // Repartition actual current pages, then reconstruct the complete closure. No
    // foreign validator certificate or missing/unused object creates the difference.
    let alternate = collect(&objects, binding, &[binding]).unwrap().container;
    alternate.admit().unwrap();
    super::super::oracle::reconstruct(&alternate).unwrap();
    alternate
}

fn source_validation_reuse_collect<S: ImmutableObjectStore + ?Sized>(
    session: &mut SourceAdmissionSession<'_, S>,
    container: &PackageContainer,
    visits: u64,
    read_bytes: u64,
) -> Result<AdmittedClosure, Diagnostic> {
    session.collect_preplanned_with_budget(
        source_validation_reuse_plan(container),
        visits,
        read_bytes,
    )
}

#[test]
fn source_validation_reuse_shares_only_complete_local_proofs_across_overlapping_roots() {
    let supplier = source_validation_reuse_fixture();
    let left = dependency_copy_importer(b"source-validation-left", &[&supplier]);
    let right = dependency_copy_importer(b"source-validation-right", &[&supplier]);
    let mut objects = left.container.objects.clone();
    objects.extend(right.container.objects.clone());
    let control = ExecutionControl::uncancelled();
    let mut session = SourceAdmissionSession::new(&objects, &control);
    source_validation_reuse_collect(
        &mut session,
        &left.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(session.completed_count(), 2);
    let before_reused = crate::platform::witness::checked_full_validation_invocations();
    let reused = source_validation_reuse_collect(
        &mut session,
        &right.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    let reused_full_validations =
        crate::platform::witness::checked_full_validation_invocations() - before_reused;
    assert_eq!(session.completed_count(), 3);
    let before_cold = crate::platform::witness::checked_full_validation_invocations();
    let cold = collect_preplanned_with_control(
        &objects,
        source_validation_reuse_plan(&right.container),
        &control,
    )
    .unwrap();
    let cold_full_validations =
        crate::platform::witness::checked_full_validation_invocations() - before_cold;
    // A tiny source can cost more after the positive modeled witness-rebuild reservation.
    // Observe that unfavorable current work honestly; semantic proof reuse is the real
    // absence of one complete full-validator invocation, not universal budget savings.
    eprintln!(
        "source_validation_reuse_overlap current_visits reused={} cold={} full_validations reused={} cold={}",
        reused.validation_visits,
        cold.validation_visits,
        reused_full_validations,
        cold_full_validations,
    );
    assert_eq!(reused_full_validations, 1);
    assert_eq!(cold_full_validations, 2);
    assert_eq!(reused.validation_read_bytes, cold.validation_read_bytes);
    assert_eq!(
        reused.container.encode().unwrap(),
        cold.container.encode().unwrap()
    );
    assert_eq!(reused.dependency_order, cold.dependency_order);
    let independent = super::super::oracle::reconstruct(&reused.container).unwrap();
    for (revision, package) in &reused.packages {
        assert_eq!(
            package.snapshot.owners,
            independent.snapshots[&package.revision.package].owners
        );
        assert_eq!(
            package.snapshot.types,
            cold.packages[revision].snapshot.types
        );
        assert_eq!(
            package.snapshot.dependency_types,
            cold.packages[revision].snapshot.dependency_types
        );
        assert_eq!(
            package.witness.manifest_digest,
            cold.packages[revision].witness.manifest_digest
        );
        assert_eq!(
            package.witness.report,
            cold.packages[revision].witness.report
        );
    }
    let right_snapshot = &reused.packages[&right.container.root.package_revision].snapshot;
    assert_eq!(right_snapshot.representation_dependencies.len(), 1);
    assert!(
        !right_snapshot.representation_dependencies.contains_key(
            &left.packages[&left.container.root.package_revision]
                .revision
                .package,
        )
    );
}

struct SourceValidationReuseMutableStore {
    objects: RefCell<BTreeMap<ObjectKey, Vec<u8>>>,
}

impl ImmutableObjectStore for SourceValidationReuseMutableStore {
    fn read(
        &self,
        key: ObjectKey,
        maximum: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_admitted(key, maximum, &mut StoreReadAdmission::unbounded(), work)
    }

    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum: usize,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.objects
            .borrow()
            .read_admitted(key, maximum, admission, work)
    }

    fn stage(
        &mut self,
        _key: ObjectKey,
        _bytes: &[u8],
        _work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(StoreError::new(
            StoreErrorClass::Input,
            "test_source_read_only",
            "read only",
        ))
    }
}

#[test]
fn source_validation_reuse_rereads_private_and_imported_objects_before_every_hit() {
    let original = source_validation_reuse_fixture();
    let blob = b"complete private blob".to_vec();
    let digest = BlobObjectDigest::of(&blob);
    let mut replacement = original.packages[&original.container.root.package_revision]
        .snapshot
        .owners
        .values()
        .find(|owner| {
            matches!(owner, OwnerRecord::Expression(record)
            if matches!(record.operation, ExpressionOperation::Text { .. }))
        })
        .unwrap()
        .clone();
    let OwnerRecord::Expression(record) = &mut replacement else {
        unreachable!()
    };
    record.operation = ExpressionOperation::Text {
        value: TextValue::Blob {
            digest,
            bytes: blob.len() as u64,
        },
    };
    // The neutral fixture encoder supplies a valid private blob representation. Independent
    // complete source admission, not the warm cache, certifies its actual canonical content.
    let mut with_blob = rehash_owner(&original, replacement);
    with_blob.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Blob, digest.bytes()),
        blob,
    );
    let supplier = with_blob.admit().unwrap();
    let source = dependency_copy_importer(b"source-validation-reread", &[&supplier]);
    assert!(
        source
            .container
            .objects
            .keys()
            .any(|key| key.domain == ObjectDomain::Blob)
    );
    let store = SourceValidationReuseMutableStore {
        objects: RefCell::new(source.container.objects.clone()),
    };
    let control = ExecutionControl::uncancelled();
    let mut session = SourceAdmissionSession::new(&store, &control);
    source_validation_reuse_collect(
        &mut session,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(session.completed_count(), 2);
    // Every object was independently present in the accepted complete container. Deleting
    // and corrupting each key covers private owners/blobs, imported types/interfaces/maps,
    // transports, exact revisions and the root. A warm proof never supplies missing bytes.
    for (key, bytes) in &source.container.objects {
        store.objects.borrow_mut().remove(key).unwrap();
        let missing = source_validation_reuse_collect(
            &mut session,
            &source.container,
            MAXIMUM_VALIDATION_VISITS,
            MAXIMUM_VALIDATION_READ_BYTES,
        )
        .unwrap_err();
        let independent_missing = collect_preplanned_with_control(
            &store,
            source_validation_reuse_plan(&source.container),
            &control,
        )
        .unwrap_err();
        assert_eq!(missing.class, independent_missing.class, "missing {key:?}");
        assert_eq!(missing.code, independent_missing.code, "missing {key:?}");
        store.objects.borrow_mut().insert(*key, bytes.clone());
        store.objects.borrow_mut().get_mut(key).unwrap().push(0xff);
        let corrupt = source_validation_reuse_collect(
            &mut session,
            &source.container,
            MAXIMUM_VALIDATION_VISITS,
            MAXIMUM_VALIDATION_READ_BYTES,
        )
        .unwrap_err();
        assert_eq!(
            corrupt.class,
            DiagnosticClass::Corrupt,
            "corrupt {key:?}: {corrupt:?}"
        );
        store.objects.borrow_mut().insert(*key, bytes.clone());
        assert_eq!(session.completed_count(), 2);
    }
    let recovered = source_validation_reuse_collect(
        &mut session,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(
        recovered.container.encode().unwrap(),
        source.container.encode().unwrap()
    );
}

#[test]
fn source_validation_reuse_exact_raw_transport_and_invalid_private_source_require_fresh_proof() {
    let source = source_validation_reuse_fixture();
    let other_transport = source_validation_reuse_alternate_transport(&source);
    assert_eq!(
        other_transport.root.package_revision,
        source.container.root.package_revision
    );
    assert_ne!(
        other_transport.root.transport,
        source.container.root.transport
    );
    let private = source.packages[&source.container.root.package_revision]
        .snapshot
        .owners
        .values()
        .find(|owner| {
            matches!(owner, OwnerRecord::Expression(record)
            if matches!(record.operation, ExpressionOperation::I64 { value: 29 }))
        })
        .unwrap()
        .clone();
    let mut replacement = private;
    let OwnerRecord::Expression(record) = &mut replacement else {
        unreachable!()
    };
    record.operation = ExpressionOperation::Bool { value: true };
    let invalid = rehash_owner(&source, replacement);
    assert_eq!(invalid.admit().unwrap_err().code, "kernel_type_if_branches");
    let mut objects = source.container.objects.clone();
    objects.extend(other_transport.objects.clone());
    objects.extend(invalid.objects.clone());
    let control = ExecutionControl::uncancelled();
    let mut session = SourceAdmissionSession::new(&objects, &control);
    source_validation_reuse_collect(
        &mut session,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    let before_warm = crate::platform::witness::checked_full_validation_invocations();
    let warm = source_validation_reuse_collect(
        &mut session,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(
        crate::platform::witness::checked_full_validation_invocations() - before_warm,
        0,
        "the exact already validated transport reuses its complete proof"
    );
    let before_changed = crate::platform::witness::checked_full_validation_invocations();
    let changed = source_validation_reuse_collect(
        &mut session,
        &other_transport,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(
        crate::platform::witness::checked_full_validation_invocations() - before_changed,
        1,
        "a different current physical transport requires a fresh complete local proof"
    );
    assert!(changed.validation_visits > warm.validation_visits);
    assert_eq!(session.completed_count(), 1);
    assert_eq!(
        changed.packages[&changed.container.root.package_revision]
            .witness
            .manifest_digest,
        warm.packages[&warm.container.root.package_revision]
            .witness
            .manifest_digest
    );
    let refused = source_validation_reuse_collect(
        &mut session,
        &invalid,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap_err();
    assert_eq!(refused.code, "kernel_type_if_branches");
    assert_eq!(session.completed_count(), 1);
}

#[test]
fn source_validation_reuse_binds_each_own_transitive_transport_selection() {
    let supplier = source_validation_reuse_fixture();
    let middle = dependency_copy_importer(b"source-validation-middle-transport", &[&supplier]);
    let source = dependency_copy_importer(b"source-validation-own-transport", &[&middle]);
    let alternate_supplier = source_validation_reuse_alternate_transport(&supplier);
    let mut alternate = source.container.clone();
    let selected = alternate
        .selections
        .iter_mut()
        .find(|binding| binding.package_revision == supplier.container.root.package_revision)
        .unwrap();
    *selected = alternate_supplier.root;
    alternate.objects.remove(&ObjectKey::from_digest(
        ObjectDomain::PackageTransport,
        supplier.container.root.transport.bytes(),
    ));
    alternate.objects.extend(alternate_supplier.objects.clone());
    assert_eq!(
        alternate.root, source.container.root,
        "the containing package's exact source is unchanged"
    );
    assert!(alternate.selections.contains(&middle.container.root));
    let mut objects = source.container.objects.clone();
    objects.extend(alternate.objects.clone());
    let control = ExecutionControl::uncancelled();
    let mut session = SourceAdmissionSession::new(&objects, &control);
    source_validation_reuse_collect(
        &mut session,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    let before_warm = crate::platform::witness::checked_full_validation_invocations();
    let warm = source_validation_reuse_collect(
        &mut session,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(
        crate::platform::witness::checked_full_validation_invocations() - before_warm,
        0,
        "the unchanged complete transport selection reuses all three proofs"
    );
    let before_changed = crate::platform::witness::checked_full_validation_invocations();
    let changed = source_validation_reuse_collect(
        &mut session,
        &alternate,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(
        crate::platform::witness::checked_full_validation_invocations() - before_changed,
        3,
        "the supplier's changed transport invalidates its own and both containing proofs"
    );
    assert!(changed.validation_visits > warm.validation_visits);
    assert_eq!(session.completed_count(), 3);
    for (revision, package) in &changed.packages {
        assert_eq!(
            package.witness.manifest_digest,
            warm.packages[revision].witness.manifest_digest
        );
        assert_eq!(
            package.snapshot.owners,
            warm.packages[revision].snapshot.owners
        );
    }
    // Independent complete wire admission must accept the actual returned new selection.
    changed.container.admit().unwrap();
}

#[test]
fn source_validation_reuse_hit_boundary_obeys_actual_control_and_failed_first_attempts() {
    let source = source_validation_reuse_fixture();
    let control = ExecutionControl::uncancelled();
    let mut session = SourceAdmissionSession::new(&source.container.objects, &control);
    source_validation_reuse_collect(
        &mut session,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    {
        let _guard = SourceFullValidationCancellation::arm();
        let error = source_validation_reuse_collect(
            &mut session,
            &source.container,
            MAXIMUM_VALIDATION_VISITS,
            MAXIMUM_VALIDATION_READ_BYTES,
        )
        .unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Cancelled);
        assert_eq!(error.code, "execution_cancelled");
        assert!(control.is_cancelled());
        assert_eq!(session.completed_count(), 1);
    }
    let cold_control = ExecutionControl::uncancelled();
    let mut cold = SourceAdmissionSession::new(&source.container.objects, &cold_control);
    {
        let _guard = SourceFullValidationCancellation::arm();
        let error = source_validation_reuse_collect(
            &mut cold,
            &source.container,
            MAXIMUM_VALIDATION_VISITS,
            MAXIMUM_VALIDATION_READ_BYTES,
        )
        .unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Cancelled);
        assert_eq!(cold.completed_count(), 0);
    }
    let fresh_control = ExecutionControl::uncancelled();
    let mut fresh = SourceAdmissionSession::new(&source.container.objects, &fresh_control);
    let recovered = source_validation_reuse_collect(
        &mut fresh,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(fresh.completed_count(), 1);
    assert_eq!(
        recovered.container.encode().unwrap(),
        source.container.encode().unwrap()
    );
}

std::thread_local! {
    static CANCEL_BEFORE_SOURCE_COMPOSED_VALIDATION: std::cell::Cell<bool> = const {
        std::cell::Cell::new(false)
    };
}

pub(super) fn before_source_composed_validation(control: &ExecutionControl) {
    if CANCEL_BEFORE_SOURCE_COMPOSED_VALIDATION.with(|cancel| cancel.replace(false)) {
        control.cancel();
    }
}

struct SourceCompositionCancellation;
impl Drop for SourceCompositionCancellation {
    fn drop(&mut self) {
        CANCEL_BEFORE_SOURCE_COMPOSED_VALIDATION.with(|cancel| cancel.set(false));
    }
}

#[test]
fn source_validation_reuse_fresh_composition_and_cancellation_do_not_publish_new_prefixes() {
    let supplier = source_validation_reuse_fixture();
    let source = dependency_copy_importer(b"source-validation-cancel-new-root", &[&supplier]);
    let control = ExecutionControl::uncancelled();
    let mut session = SourceAdmissionSession::new(&source.container.objects, &control);
    source_validation_reuse_collect(
        &mut session,
        &supplier.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(session.completed_count(), 1);
    let _guard = SourceCompositionCancellation;
    CANCEL_BEFORE_SOURCE_COMPOSED_VALIDATION.with(|cancel| assert!(!cancel.replace(true)));
    let cancelled = source_validation_reuse_collect(
        &mut session,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap_err();
    assert_eq!(cancelled.class, DiagnosticClass::Cancelled);
    assert_eq!(cancelled.code, "execution_cancelled");
    assert!(control.is_cancelled());
    assert_eq!(
        session.completed_count(),
        1,
        "new root token must remain unpublished"
    );
    assert!(CANCEL_BEFORE_SOURCE_COMPOSED_VALIDATION.with(|cancel| !cancel.get()));
    let fresh_control = ExecutionControl::uncancelled();
    let mut fresh_session = SourceAdmissionSession::new(&source.container.objects, &fresh_control);
    let recovered = source_validation_reuse_collect(
        &mut fresh_session,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(fresh_session.completed_count(), 2);
    assert_eq!(
        recovered.container.encode().unwrap(),
        source.container.encode().unwrap()
    );
}

#[test]
fn source_validation_reuse_keeps_cumulative_exact_fit_and_capacity_skip_budgets() {
    let source = source_validation_reuse_fixture();
    let control = ExecutionControl::uncancelled();
    let mut measured = SourceAdmissionSession::new(&source.container.objects, &control);
    let cold = source_validation_reuse_collect(
        &mut measured,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    let warm = source_validation_reuse_collect(
        &mut measured,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    let aggregate_visits = cold.validation_visits + warm.validation_visits;
    let aggregate_bytes = cold.validation_read_bytes + warm.validation_read_bytes;
    for (visits, bytes, succeeds) in [
        (aggregate_visits, aggregate_bytes, true),
        (aggregate_visits - 1, aggregate_bytes, false),
        (aggregate_visits, aggregate_bytes - 1, false),
    ] {
        let mut session = SourceAdmissionSession::new(&source.container.objects, &control);
        let first = source_validation_reuse_collect(&mut session, &source.container, visits, bytes)
            .unwrap();
        let second = source_validation_reuse_collect(
            &mut session,
            &source.container,
            visits - first.validation_visits,
            bytes - first.validation_read_bytes,
        );
        assert_eq!(second.is_ok(), succeeds);
        if let Err(error) = second {
            assert_eq!(error.class, DiagnosticClass::Resource);
        }
        assert_eq!(session.completed_count(), 1);
    }
    let mut no_capacity =
        SourceAdmissionSession::with_retained_byte_limit(&source.container.objects, &control, 0);
    let first = source_validation_reuse_collect(
        &mut no_capacity,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    let second = source_validation_reuse_collect(
        &mut no_capacity,
        &source.container,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
    .unwrap();
    assert_eq!(no_capacity.completed_count(), 0);
    assert_eq!(first.validation_visits, second.validation_visits);
    assert_eq!(first.validation_read_bytes, second.validation_read_bytes);
    assert_eq!(
        first.container.encode().unwrap(),
        second.container.encode().unwrap()
    );
}
