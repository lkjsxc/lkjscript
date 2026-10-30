use super::*;

struct NoReads;

impl CodeRead for NoReads {
    fn code_dependency(&self, _: PackageId) -> Result<CanonicalRead<bool>, Diagnostic> {
        panic!("type retention must not read source metadata")
    }
    fn code_owner(&self, _: OwnerKey) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        panic!("type retention must not read source metadata")
    }
    fn code_type(
        &self,
        _: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        panic!("type retention must not read source metadata")
    }
    fn code_interface(
        &self,
        _: PackageId,
        _: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        panic!("type retention must not read source metadata")
    }
}

#[test]
fn cumulative_derived_type_admission_reuses_exact_entries_and_rejects_growth() {
    let mut builder = UnitBuilder {
        canonical: &NoReads,
        package: PackageId::from_bytes([1; 16]).unwrap(),
        scope: None,
        tables: TablesBuilder::default(),
        derived_type_objects: BTreeMap::new(),
        derived_type_admission: DerivedTypeAdmission::default(),
        work: CompilationWork::default(),
    };
    let first = TypeObject::new(TypeForm::I64).unwrap();
    let digest = builder.retain_derived_type(None, &first).unwrap();
    let retained = builder.derived_type_admission.bytes;
    assert!(retained.consumed > 0);
    assert!(!retained.saturated);
    builder.derived_type_admission.maximum_bytes = Some(retained.consumed);
    assert_eq!(
        builder.retain_derived_type(Some(digest), &first).unwrap(),
        digest
    );
    assert_eq!(builder.derived_type_admission.bytes, retained);
    assert_eq!(builder.derived_type_objects.len(), 1);
    let error = builder
        .retain_derived_type(None, &TypeObject::new(TypeForm::Bool).unwrap())
        .expect_err(
            "individually small generated types cannot bypass the cumulative storage bound",
        );
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "compiler_derived_type_storage");
    assert_eq!(builder.derived_type_admission.bytes, retained);
    assert_eq!(builder.derived_type_objects.len(), 1);
}

#[test]
fn derived_metadata_default_crosses_former_limit_and_observes_saturation() {
    let mut builder = UnitBuilder {
        canonical: &NoReads,
        package: PackageId::from_bytes([1; 16]).unwrap(),
        scope: None,
        tables: TablesBuilder::default(),
        derived_type_objects: BTreeMap::new(),
        derived_type_admission: DerivedTypeAdmission::default(),
        work: CompilationWork::default(),
    };
    assert_eq!(builder.derived_type_admission.maximum_bytes, None);
    builder.derived_type_admission.bytes = ValidationWork::exact(64 * 1_048_576);
    builder
        .retain_derived_type(None, &TypeObject::new(TypeForm::I64).unwrap())
        .unwrap();
    assert!(builder.derived_type_admission.bytes.consumed > 64 * 1_048_576);
    assert!(!builder.derived_type_admission.bytes.saturated);
    builder.derived_type_admission.bytes = ValidationWork::exact(usize::MAX);
    builder
        .retain_derived_type(None, &TypeObject::new(TypeForm::Bool).unwrap())
        .unwrap();
    assert_eq!(builder.derived_type_admission.bytes.consumed, usize::MAX);
    assert!(builder.derived_type_admission.bytes.saturated);
    builder.derived_type_admission.maximum_bytes = Some(usize::MAX);
    let before = builder.derived_type_admission.bytes;
    assert_eq!(
        builder
            .retain_derived_type(None, &TypeObject::new(TypeForm::Unit).unwrap())
            .unwrap_err()
            .code,
        "compiler_derived_type_storage"
    );
    assert_eq!(builder.derived_type_admission.bytes, before);
    assert_eq!(builder.derived_type_objects.len(), 2);
}

#[test]
fn compiler_logical_admission_preserves_zero_exact_maximum_and_failed_preflight() {
    for maximum in [0, 1, usize::MAX] {
        let admission = CompilerWorkAdmission::with_limit(Some(maximum));
        admission
            .reserve(maximum, "compiler_parallel_proof_work", "exact proof")
            .unwrap();
        assert_eq!(admission.observation(), ValidationWork::exact(maximum));
        let before = admission.observation();
        assert_eq!(
            admission
                .reserve(1, "compiler_parallel_proof_work", "one over proof")
                .unwrap_err()
                .code,
            "compiler_parallel_proof_work"
        );
        assert_eq!(admission.observation(), before);
    }
}
