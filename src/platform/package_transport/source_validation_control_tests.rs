// Included in source.rs tests. The phase probe cancels the real owning control, without
// changing canonical inputs or installing a different source validation implementation.

#[test]
fn source_admission_unbounded_observations_do_not_exhaust_permission() {
    let bytes = b"source admission observation".to_vec();
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
    let objects = BTreeMap::from([(key, bytes.clone())]);
    let store = CollectingStore::new(&objects);
    store.charge_visits(FORMER_VALIDATION_VISITS + 1).unwrap();
    assert_eq!(store.visits(), FORMER_VALIDATION_VISITS + 1);
    assert!(!store.visits_saturated());
    assert_eq!(store.remaining_kernel_work().unwrap(), None);
    store.charge_visits(u64::MAX).unwrap();
    assert!(store.visits_saturated());
    store.charge_visits(1).unwrap();
    // Seed a prior read observation without allocating its represented bytes. This is a
    // scalar boundary control, not evidence of a physically large package admission.
    store
        .admission
        .borrow_mut()
        .admit_object_bytes(u64::MAX)
        .unwrap();
    assert!(!store.read_bytes_saturated());
    let read = store.read(key, None, &mut StoreWork::default()).unwrap();
    assert_eq!(read, Some(bytes.clone()));
    assert_eq!(store.read_bytes(), u64::MAX);
    assert!(store.read_bytes_saturated());
    assert_eq!(store.remaining_visits(), None);
    assert_eq!(store.remaining_read_bytes(), None);
    assert_eq!(
        store.read(key, None, &mut StoreWork::default()).unwrap(),
        Some(bytes)
    );
    assert_eq!(store.objects.borrow().len(), 1);
}

#[test]
fn source_admission_maximum_finite_visit_and_byte_allowances_remain_exact() {
    let bytes = b"finite source admission".to_vec();
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
    let objects = BTreeMap::from([(key, bytes.clone())]);
    let store = CollectingStore::with_budget(&objects, Some(u64::MAX), None);
    store.charge_visits(u64::MAX - 1).unwrap();
    let mut outer = StoreReadAdmission::new(StoreReadLimits {
        maximum_catalog_lookups: Some(2),
        maximum_objects: Some(2),
        maximum_bytes: Some(bytes.len() as u64 * 2),
    });
    assert_eq!(
        store
            .read_admitted(key, None, &mut outer, &mut StoreWork::default())
            .unwrap(),
        Some(bytes.clone())
    );
    assert_eq!(store.visits(), u64::MAX);
    assert!(!store.visits_saturated());
    assert_eq!(store.remaining_visits(), Some(0));
    assert_eq!(outer.remaining().maximum_objects, Some(1));
    let error = store
        .read_admitted(key, None, &mut outer, &mut StoreWork::default())
        .unwrap_err();
    assert_eq!(error.code, "object_read_objects_exhausted");
    // The failed read reserved one catalog lookup, but never reserved or copied an object.
    assert_eq!(outer.remaining().maximum_catalog_lookups, Some(0));
    assert_eq!(outer.remaining().maximum_objects, Some(1));
    assert_eq!(outer.remaining().maximum_bytes, Some(bytes.len() as u64));
    assert_eq!(store.visits(), u64::MAX);
    assert_eq!(store.objects.borrow().len(), 1);
    assert!(store.charge_visits(1).is_err());

    let store = CollectingStore::with_budget(&objects, None, Some(u64::MAX));
    store.charge_visits(u64::MAX).unwrap();
    store.charge_visits(1).unwrap();
    assert!(store.visits_saturated());
    store
        .admission
        .borrow_mut()
        .admit_object_bytes(u64::MAX - bytes.len() as u64)
        .unwrap();
    assert_eq!(store.remaining_read_bytes(), Some(bytes.len() as u64));
    assert_eq!(
        store.read(key, None, &mut StoreWork::default()).unwrap(),
        Some(bytes)
    );
    assert_eq!(store.read_bytes(), u64::MAX);
    assert!(!store.read_bytes_saturated());
    assert_eq!(store.remaining_read_bytes(), Some(0));
    assert_eq!(
        store
            .read(key, None, &mut StoreWork::default())
            .unwrap_err()
            .code,
        "object_read_bytes_exhausted"
    );
    assert_eq!(store.remaining_read_bytes(), Some(0));
}

#[test]
fn source_admission_joint_read_failure_commits_only_exact_prior_reservations() {
    let bytes = b"joint source admission".to_vec();
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
    let objects = BTreeMap::from([(key, bytes.clone())]);
    let store = CollectingStore::with_budget(&objects, Some(2), Some(bytes.len() as u64));
    store.charge_visits(1).unwrap();
    let mut outer = StoreReadAdmission::new(StoreReadLimits {
        maximum_catalog_lookups: Some(1),
        maximum_objects: Some(1),
        maximum_bytes: Some(bytes.len() as u64 - 1),
    });
    let error = store
        .read_admitted(key, None, &mut outer, &mut StoreWork::default())
        .unwrap_err();
    assert_eq!(error.code, "object_read_bytes_exhausted");
    assert_eq!(outer.remaining().maximum_catalog_lookups, Some(0));
    assert_eq!(outer.remaining().maximum_objects, Some(1));
    assert_eq!(outer.remaining().maximum_bytes, Some(bytes.len() as u64 - 1));
    assert_eq!(store.remaining_visits(), Some(1));
    assert_eq!(store.remaining_read_bytes(), Some(bytes.len() as u64));
    assert_eq!(store.visits(), 1);
    assert!(store.objects.borrow().is_empty());
    assert_eq!(
        store.read(key, None, &mut StoreWork::default()).unwrap(),
        Some(bytes)
    );
    assert_eq!(store.remaining_visits(), Some(0));
    assert_eq!(store.remaining_read_bytes(), Some(0));
    assert_eq!(store.visits(), 2);
}

std::thread_local! {
    static CANCEL_BEFORE_SOURCE_FULL_VALIDATION: std::cell::Cell<bool> = const {
        std::cell::Cell::new(false)
    };
}

pub(super) fn before_source_full_validation(control: &ExecutionControl) {
    if CANCEL_BEFORE_SOURCE_FULL_VALIDATION.with(|cancel| cancel.replace(false)) {
        control.cancel();
    }
}

struct SourceFullValidationCancellation;

impl SourceFullValidationCancellation {
    fn arm() -> Self {
        CANCEL_BEFORE_SOURCE_FULL_VALIDATION.with(|cancel| assert!(!cancel.replace(true)));
        Self
    }
}

impl Drop for SourceFullValidationCancellation {
    fn drop(&mut self) {
        CANCEL_BEFORE_SOURCE_FULL_VALIDATION.with(|cancel| cancel.set(false));
    }
}

#[test]
fn source_admission_cancellation_reaches_full_validation_before_late_semantic_refusal_and_recovers()
{
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create controlled (function create choose (visibility public) (parameter create flag (type Bool)) (returns I64) (effect pure) (body (if (local flag) (i64 17) (i64 23))))))\ndeclarations.end\n",
    ).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let source = GraphRepository::create(&directory.path().join("source"), &snapshot, None)
        .unwrap()
        .repository
        .export_package_container()
        .unwrap();
    let exact = source.container.encode().unwrap();
    let mut replacement = source.packages[&source.container.root.package_revision].snapshot.owners.values()
        .find(|owner| matches!(owner, OwnerRecord::Expression(expression) if matches!(expression.operation, ExpressionOperation::I64 { value: 17 })))
        .unwrap().clone();
    let OwnerRecord::Expression(expression) = &mut replacement else {
        unreachable!()
    };
    expression.operation = ExpressionOperation::Bool { value: true };
    // Neutral hostile rehash retains the exact valid public signature while changing one
    // private branch's canonical meaning. Real full inference must reject the branch mismatch.
    let mutant = rehash_owner(&source, replacement);
    let semantic = mutant.admit().unwrap_err();
    assert_eq!(semantic.class, DiagnosticClass::Semantic);
    assert_eq!(semantic.code, "kernel_type_if_branches");
    let control = ExecutionControl::uncancelled();
    {
        let _probe = SourceFullValidationCancellation::arm();
        let cancelled = mutant.admit_with_control(&control).unwrap_err();
        assert!(control.is_cancelled());
        assert_eq!(cancelled.class, DiagnosticClass::Cancelled);
        assert_eq!(cancelled.code, "execution_cancelled");
        assert!(CANCEL_BEFORE_SOURCE_FULL_VALIDATION.with(|cancel| !cancel.get()));
    }
    // Cancellation owns this attempt; it neither certifies the mutant nor poisons fresh controls.
    assert_eq!(mutant.admit().unwrap_err().code, "kernel_type_if_branches");
    let recovered = source
        .container
        .admit_with_control(&ExecutionControl::uncancelled())
        .unwrap();
    let revision = source.container.root.package_revision;
    assert_eq!(recovered.container.encode().unwrap(), exact);
    assert_eq!(
        recovered.validation_visits,
        source.validation_visits + source.container.objects.len() as u64
    );
    assert_eq!(
        recovered.validation_read_bytes,
        source.validation_read_bytes + exact.len() as u64
    );
    assert_eq!(
        recovered.packages[&revision].witness.manifest_digest,
        source.packages[&revision].witness.manifest_digest
    );
    assert_eq!(
        recovered.packages[&revision].snapshot.owners,
        source.packages[&revision].snapshot.owners
    );
    assert_eq!(source.container.encode().unwrap(), exact);
}
