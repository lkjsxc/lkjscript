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
    assert!(retained > 0);
    builder.derived_type_admission.maximum_bytes = retained;
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
