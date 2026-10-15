// Included by the existing source-loader test owner to share its neutral hostile rehasher.

fn f64_test_source() -> (
    AdmittedClosure,
    crate::platform::kernel::tests::f64_admission_tests::AdmissionIds,
) {
    let (snapshot, ids) =
        crate::platform::kernel::tests::f64_admission_tests::admission_snapshot(false);
    let temporary = tempfile::tempdir().unwrap();
    let repository =
        GraphRepository::create(&temporary.path().join("source"), &snapshot, None).unwrap();
    (
        repository.repository.export_package_container().unwrap(),
        ids,
    )
}

fn assert_f64_source_rejected(hostile: &PackageContainer, expected: &str) {
    let bytes = hostile.encode().unwrap();
    let decoded = PackageContainer::decode(&bytes, hostile.root.transport).unwrap();
    assert_eq!(decoded.admit().unwrap_err().code, expected);
    let error = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap_err();
    assert!(
        error.code == expected || error.message.contains(expected),
        "{error:?}"
    );
    assert_not_ready(&bytes, hostile.root.transport);
}

#[test]
fn f64_private_external_signatures_cannot_bypass_independent_package_source_admission() {
    let (original, ids) = f64_test_source();
    let package = &original.packages[&original.container.root.package_revision];
    // This private declaration is never called. Its Option<Unit> signature is valid
    // for core.option.none and foreign to every new numeric intrinsic, independently of exports.
    for implementation in [
        "core.f64.add",
        "core.f64.subtract",
        "core.f64.multiply",
        "core.f64.divide",
        "core.f64.negate",
        "core.f64.abs",
        "core.f64.sqrt",
        "core.f64.less",
        "core.f64.less-equal",
        "core.f64.is-finite",
        "core.f64.is-nan",
        "core.f64.from-i64",
        "core.f64.to-i64-result",
        "core.f64.parse-result",
        "core.f64.to-text",
    ] {
        let mut replacement = package.snapshot.owners[&OwnerKey::Declaration(ids.external)].clone();
        let OwnerRecord::Declaration(declaration) = &mut replacement else {
            panic!("external declaration");
        };
        let DeclarationPayload::External(external) = &mut declaration.payload else {
            panic!("external payload");
        };
        external.implementation = ImplementationName::new(implementation).unwrap();
        assert_f64_source_rejected(&rehash_owner(&original, replacement), "intrinsic_signature");
    }
}

#[test]
fn f64_unreachable_branch_and_unused_type_faults_reject_before_package_readiness() {
    use crate::platform::binary64::Binary64;
    let (original, ids) = f64_test_source();
    let package = &original.packages[&original.container.root.package_revision];
    let mut replacement = package.snapshot.owners[&OwnerKey::Expression(ids.unchosen)].clone();
    let OwnerRecord::Expression(expression) = &mut replacement else {
        panic!("unchosen arm");
    };
    expression.operation = ExpressionOperation::F64 {
        value: Binary64::from_bits(0x7ff8_0000_0000_0000).unwrap(),
    };
    assert_f64_source_rejected(
        &rehash_owner(&original, replacement),
        "kernel_type_if_branches",
    );

    // An unused parameter remains a complete type-closure admission root. Rehashing
    // every enclosing source commitment cannot hide a malformed nested scalar object.
    for malformed_tag in [false, true] {
        let scalar_bytes = if malformed_tag {
            crate::platform::packed::encode_canonical(
                contract::TYPE_OBJECT_MAGIC,
                contract::TYPE_OBJECT_ENVELOPE_DOMAIN,
                &(contract::TYPE_OBJECT_CONTRACT_VERSION, u32::MAX),
                contract::MAXIMUM_TYPE_OBJECT_BYTES,
            )
            .unwrap()
        } else {
            crate::platform::packed::encode_canonical(
                contract::TYPE_OBJECT_MAGIC,
                contract::TYPE_OBJECT_ENVELOPE_DOMAIN,
                &TypeObject {
                    contract_version: 0,
                    form: TypeForm::F64,
                },
                contract::MAXIMUM_TYPE_OBJECT_BYTES,
            )
            .unwrap()
        };
        let scalar = TypeObjectDigest::of(&scalar_bytes);
        let (list, list_bytes) =
            encode_type_object(&TypeObject::new(TypeForm::List { item: scalar }).unwrap()).unwrap();
        let mut replacement = package.snapshot.owners[&OwnerKey::Parameter(ids.unused)].clone();
        let OwnerRecord::Parameter(parameter) = &mut replacement else {
            panic!("unused parameter");
        };
        let old = std::mem::replace(&mut parameter.ty, list);
        let mut hostile = rehash_owner(&original, replacement);
        hostile
            .objects
            .remove(&ObjectKey::from_digest(ObjectDomain::Type, old.bytes()));
        hostile.objects.insert(
            ObjectKey::from_digest(ObjectDomain::Type, scalar.bytes()),
            scalar_bytes,
        );
        hostile.objects.insert(
            ObjectKey::from_digest(ObjectDomain::Type, list.bytes()),
            list_bytes,
        );
        assert_f64_source_rejected(
            &hostile,
            if malformed_tag {
                "packed_decode"
            } else {
                "kernel_type_contract"
            },
        );
    }
}

#[test]
fn current_f64_source_compiles_and_executes_differentially_without_rewriting_meaning() {
    use crate::platform::compiler::{compile_immutable, load_artifact};
    use crate::platform::execution::normalized::{
        NormalizedProgram, NormalizedRunPolicy, run_graph_tests,
    };
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create numeric (function create value (visibility public) (returns F64) (effect pure) (body (f64 1.25))) (test create precise (visibility private) (actual (call value)) (expected (f64 1.25)))))\ndeclarations.end\n"
    ).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let original = GraphRepository::create(&temporary.path().join("numeric"), &snapshot, None)
        .unwrap()
        .repository
        .export_package_container()
        .unwrap();
    let bytes = original.container.encode().unwrap();
    let container = PackageContainer::decode(&bytes, original.container.root.transport).unwrap();
    let admitted = container.admit().unwrap();
    let package = &admitted.packages[&container.root.package_revision];
    let independent = crate::platform::package_transport::oracle::reconstruct(&container).unwrap();
    assert_eq!(
        package.snapshot.owners,
        independent.snapshots[&package.snapshot.root.package_id].owners
    );
    assert_eq!(
        package.snapshot.types,
        independent.snapshots[&package.snapshot.root.package_id].types
    );
    let compiled = compile_immutable(package, &container.objects, &[]).unwrap();
    let program = std::sync::Arc::new(
        NormalizedProgram::prepare(load_artifact(&compiled.artifact.bytes).unwrap()).unwrap(),
    );
    let receipt = run_graph_tests(
        &package.snapshot,
        &program,
        None,
        NormalizedRunPolicy::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(receipt.passed, 1);
    assert_eq!(receipt.failed, 0);
    assert_eq!(receipt.differential, "equal");
    assert_eq!(container.objects, original.container.objects);
    assert_eq!(container.encode().unwrap(), bytes);
}
