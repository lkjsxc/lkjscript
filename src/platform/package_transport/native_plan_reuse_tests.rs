// Included in source.rs tests. Count actual VM calls per owning test thread;
// identical receipts alone cannot distinguish reuse from repeated evaluation.

fn native_plan_reuse_count() -> usize {
    crate::platform::package_transport::native_plan::invocation_count()
}

fn native_plan_reuse_validation(
    source: &AdmittedClosure,
) -> super::super::PackageTransportClosureValidation {
    crate::platform::package_transport::validate_package_transport_closure(
        &source.container.objects,
        source.container.root.package_revision,
        &source.container.selections,
        None,
        &mut StoreWork::default(),
    )
    .unwrap()
}

#[test]
fn native_plan_reuse_compiler_export_and_repository_helpers_invoke_once() {
    use crate::platform::change::{AuthoredChange, AuthoredChangeSet, ChangeBudget};
    use crate::platform::publication::PublicationOptions;
    let directory = tempfile::tempdir().unwrap();
    let supplier = GraphRepository::create(
        &directory.path().join("supplier"),
        &dependency_copy_empty(b"native-reuse-supplier"),
        None,
    )
    .unwrap();
    let before = native_plan_reuse_count();
    let source = supplier.repository.export_package_container().unwrap();
    assert_eq!(native_plan_reuse_count() - before, 1);
    let literal = source.container.encode().unwrap();
    let before = native_plan_reuse_count();
    let exported = supplier.repository.export_package_transport().unwrap();
    assert_eq!(native_plan_reuse_count() - before, 1);
    assert_eq!(exported.container, literal);
    let before = native_plan_reuse_count();
    let standalone = PackageContainer::decode(&literal, exported.transport_digest)
        .unwrap()
        .admit()
        .unwrap();
    assert_eq!(native_plan_reuse_count() - before, 1);
    assert_eq!(standalone.container.objects, source.container.objects);
    assert_eq!(standalone.dependency_order, source.dependency_order);

    let target = GraphRepository::create(
        &directory.path().join("consumer"),
        &dependency_copy_empty(b"native-reuse-consumer"),
        None,
    )
    .unwrap();
    target
        .repository
        .stage_package_transport(exported.transport_digest, &literal)
        .unwrap();
    let before = native_plan_reuse_count();
    let staged = target
        .repository
        .staged_package_source(exported.revision_digest)
        .unwrap();
    assert_eq!(native_plan_reuse_count() - before, 1);
    assert_eq!(staged.container.encode().unwrap(), literal);

    let request = AuthoredChangeSet {
        base: target.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::AddDependency {
            package: exported.revision.package,
            semantic_revision: exported.revision.revision.revision_id().unwrap(),
            package_revision: exported.revision_digest,
        }],
    };
    let before = native_plan_reuse_count();
    let prepared = target
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    // Added-interface resolution under canonical read admission, logical
    // authority staging, and complete candidate source admission are three
    // distinct phases. Each source selection reuses its own admitted plan.
    assert_eq!(native_plan_reuse_count() - before, 3);
    let before = native_plan_reuse_count();
    target.repository.publish(&prepared.publication).unwrap();
    assert_eq!(native_plan_reuse_count() - before, 1);

    let before = native_plan_reuse_count();
    let compiled = crate::platform::normalized_lifecycle::prepare_repository_with_control(
        target.repository.clone(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(native_plan_reuse_count() - before, 1);
    assert_eq!(
        (
            compiled.dependency_plan.packages,
            compiled.dependency_plan.edges
        ),
        (2, 1)
    );
    let source = target.repository.export_package_container().unwrap();
    assert_eq!(
        compiled.dependency_plan.input_sha256,
        source.dependency_plan.input_sha256
    );
    assert_eq!(
        compiled.dependency_plan.outcome_sha256,
        source.dependency_plan.outcome_sha256
    );
    assert_eq!(
        compiled.dependency_plan.order_sha256,
        source.dependency_plan.order_sha256
    );
}

#[test]
fn native_plan_reuse_preserves_complete_objects_and_exact_source_budgets() {
    let (source, _, _) = dependency_copy_triangle(
        "declarations.begin\n(units (module create supplier (function create value (visibility public) (returns Unit) (effect pure) (body (unit)))))\ndeclarations.end\n",
    );
    let prior = native_plan_reuse_validation(&source);
    let control = ExecutionControl::uncancelled();
    let before = native_plan_reuse_count();
    let fresh = collect_with_control(
        &source.container.objects,
        source.container.root,
        &source.container.selections,
        &control,
    )
    .unwrap();
    assert_eq!(native_plan_reuse_count() - before, 1);
    let before = native_plan_reuse_count();
    let reused = collect_preplanned_with_budget_and_control(
        &source.container.objects,
        prior.clone(),
        fresh.validation_visits,
        fresh.validation_read_bytes,
        &control,
    )
    .unwrap();
    assert_eq!(native_plan_reuse_count(), before);
    assert_eq!(reused.validation_visits, fresh.validation_visits);
    assert_eq!(reused.validation_read_bytes, fresh.validation_read_bytes);
    assert_eq!(reused.container.objects, source.container.objects);
    assert_eq!(
        reused.container.encode().unwrap(),
        fresh.container.encode().unwrap()
    );
    assert_eq!(reused.dependency_order, fresh.dependency_order);
    assert_eq!(
        reused.dependency_plan.order_sha256,
        fresh.dependency_plan.order_sha256
    );
    let visits_refusal = collect_preplanned_with_budget_and_control(
        &source.container.objects,
        prior.clone(),
        fresh.validation_visits - 1,
        fresh.validation_read_bytes,
        &control,
    )
    .unwrap_err();
    let read_bytes_refusal = collect_preplanned_with_budget_and_control(
        &source.container.objects,
        prior,
        fresh.validation_visits,
        fresh.validation_read_bytes - 1,
        &control,
    )
    .unwrap_err();
    assert_eq!(
        visits_refusal.class,
        DiagnosticClass::Resource,
        "one visit below exact {} visits / {} bytes: {visits_refusal:?}; read-byte result: {read_bytes_refusal:?}",
        fresh.validation_visits,
        fresh.validation_read_bytes,
    );
    assert_eq!(
        read_bytes_refusal.class,
        DiagnosticClass::Resource,
        "one read byte below exact {} visits / {} bytes: {read_bytes_refusal:?}",
        fresh.validation_visits,
        fresh.validation_read_bytes,
    );
    assert_eq!(native_plan_reuse_count(), before);
}

#[test]
fn native_plan_reuse_consults_private_context_and_rejects_other_exact_roots() {
    let leaf = native_plan_empty_package(b"native-reuse-private-leaf");
    let source = dependency_copy_importer(b"native-reuse-private-root", &[&leaf]);
    let mut prior = native_plan_reuse_validation(&source);
    // These descriptive projections can be changed by a crate consumer. They
    // cannot modify the original private, complete native planning context.
    prior.root_revision.interface = PackageInterfaceDigest::from_bytes([0xee; 32]);
    prior.root_transport.semantic_root = SemanticRootDigest::from_bytes([0xef; 32]);
    prior.representation_revisions.clear();
    prior.representation_interfaces.clear();
    prior.dependency_edges = usize::MAX;
    let before = native_plan_reuse_count();
    let reused = collect_preplanned_with_control(
        &source.container.objects,
        prior.clone(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(native_plan_reuse_count(), before);
    assert_eq!(reused.container.objects, source.container.objects);
    assert_eq!((reused.packages.len(), reused.dependency_edges), (2, 1));

    let other = native_plan_empty_package(b"native-reuse-other-root");
    let before = native_plan_reuse_count();
    let error = super::super::revalidate_package_revision_closure_with_control(
        &other.container.objects,
        other.container.root.package_revision,
        None,
        &prior.logical,
        &mut StoreWork::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "package_revision_plan_binding");
    assert_eq!(native_plan_reuse_count(), before);
}

#[test]
fn native_plan_reuse_compares_every_private_revision_and_edge_count() {
    let leaf = native_plan_empty_package(b"native-reuse-compare-leaf");
    let source = dependency_copy_importer(b"native-reuse-compare-root", &[&leaf]);
    let prior = native_plan_reuse_validation(&source);
    let before = native_plan_reuse_count();
    // Test-only hostile alteration of otherwise inaccessible private metadata
    // exercises the full equality guard; ordinary consumers cannot do this.
    let mut changes = [prior.clone(), prior.clone(), prior.clone(), prior.clone()];
    changes[0].logical.dependency_edges += 1;
    changes[1].logical.root_revision.interface = PackageInterfaceDigest::from_bytes([0xf0; 32]);
    changes[2]
        .logical
        .revisions
        .remove(&leaf.container.root.package_revision);
    changes[3]
        .logical
        .revisions
        .get_mut(&leaf.container.root.package_revision)
        .unwrap()
        .interface = PackageInterfaceDigest::from_bytes([0xf1; 32]);
    for changed in changes {
        let error = collect_preplanned_with_control(
            &source.container.objects,
            changed,
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert_eq!(error.code, "package_revision_plan_binding");
    }
    assert_eq!(native_plan_reuse_count(), before);
}

#[test]
fn native_plan_reuse_rereads_missing_corrupt_revisions_and_private_bodies() {
    let (source, supplier, _) = dependency_copy_triangle(
        "declarations.begin\n(units (module create supplier (function create value (visibility public) (returns Unit) (effect pure) (body (unit)))))\ndeclarations.end\n",
    );
    let prior = native_plan_reuse_validation(&source);
    let before = native_plan_reuse_count();
    for revision in source.packages.keys() {
        let key = ObjectKey::from_digest(ObjectDomain::PackageRevision, revision.bytes());
        let mut missing = source.container.objects.clone();
        missing.remove(&key).unwrap();
        assert_eq!(
            collect_preplanned_with_control(
                &missing,
                prior.clone(),
                &ExecutionControl::uncancelled()
            )
            .unwrap_err()
            .code,
            "package_revision_missing"
        );
        let mut corrupt = source.container.objects.clone();
        corrupt.get_mut(&key).unwrap().push(0xff);
        assert_eq!(
            collect_preplanned_with_control(
                &corrupt,
                prior.clone(),
                &ExecutionControl::uncancelled()
            )
            .unwrap_err()
            .class,
            DiagnosticClass::Corrupt
        );
    }
    let private = source.packages[&supplier]
        .snapshot
        .owners
        .values()
        .find(|owner| matches!(owner, OwnerRecord::Expression(_)))
        .unwrap();
    let mut missing = source.container.objects.clone();
    missing
        .remove(&ObjectKey::from_digest(
            ObjectDomain::Owner,
            encode_owner(private).unwrap().0.bytes(),
        ))
        .unwrap();
    assert_eq!(
        collect_preplanned_with_control(&missing, prior, &ExecutionControl::uncancelled())
            .unwrap_err()
            .code,
        "package_source_missing"
    );
    assert_eq!(native_plan_reuse_count(), before);
}

#[test]
fn native_plan_reuse_cancellation_reaches_canonical_reread_and_recovers() {
    struct CancelOnRead<'a> {
        base: &'a BTreeMap<ObjectKey, Vec<u8>>,
        control: ExecutionControl,
        reads: std::cell::Cell<usize>,
    }
    impl ImmutableObjectStore for CancelOnRead<'_> {
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
            let result = self.base.read_admitted(key, maximum, admission, work);
            self.reads.set(self.reads.get() + 1);
            self.control.cancel();
            result
        }
        fn stage(
            &mut self,
            _key: ObjectKey,
            _bytes: &[u8],
            _work: &mut StoreWork,
        ) -> Result<StageOutcome, StoreError> {
            Err(StoreError::new(
                StoreErrorClass::Input,
                "package_source_read_only",
                "test source is immutable",
            ))
        }
    }
    let source = native_plan_empty_package(b"native-reuse-cancel-source");
    let prior = native_plan_reuse_validation(&source);
    let control = ExecutionControl::uncancelled();
    let store = CancelOnRead {
        base: &source.container.objects,
        control: control.clone(),
        reads: std::cell::Cell::new(0),
    };
    let before = native_plan_reuse_count();
    let error = collect_preplanned_with_control(&store, prior.clone(), &control).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(error.code, "execution_cancelled");
    assert!(store.reads.get() > 0);
    assert_eq!(native_plan_reuse_count(), before);
    let recovered = collect_preplanned_with_control(
        &source.container.objects,
        prior,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(recovered.container.objects, source.container.objects);
    assert_eq!(recovered.dependency_order, source.dependency_order);
    assert_eq!(native_plan_reuse_count(), before);
}
