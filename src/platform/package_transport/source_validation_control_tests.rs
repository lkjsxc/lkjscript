// Included in source.rs tests. The phase probe cancels the real owning control, without
// changing canonical inputs or installing a different source validation implementation.
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
