// Included by the existing source-loader test owner to share its neutral hostile rehasher.

fn f64_test_source(
    generation: u16,
) -> (
    AdmittedClosure,
    crate::platform::kernel::tests::f64_admission_tests::AdmissionIds,
) {
    let (snapshot, ids) =
        crate::platform::kernel::tests::f64_admission_tests::admission_snapshot(false, generation);
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
    let (original, ids) = f64_test_source(17);
    let package = &original.packages[&original.container.root.package_revision];
    // This private declaration is never called. Its predecessor Option<Unit> signature is valid
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
    let (original, ids) = f64_test_source(17);
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

    // An unused parameter is still a complete type-closure admission root. The parameter and all
    // source commitments are coherently rehashed; neither public interfaces nor runtime reachability
    // can hide a malformed new scalar envelope or a scalar smuggled into the predecessor graph.
    for generation in [16, 17] {
        let (original, ids) = f64_test_source(generation);
        let package = &original.packages[&original.container.root.package_revision];
        for malformed_tag in [false, true] {
            if generation == 17 && !malformed_tag {
                continue;
            }
            let (float, float_bytes) = if malformed_tag {
                let bytes = crate::platform::packed::encode(
                    *b"LKJF6401",
                    "lkjscript.kernel.f64-type-envelope.v1",
                    &(1_u16, 2_u8),
                    crate::platform::kernel::contract::MAXIMUM_TYPE_OBJECT_BYTES,
                )
                .unwrap();
                (TypeObjectDigest::of(&bytes), bytes)
            } else {
                encode_type_object(&TypeObject::new(TypeForm::F64).unwrap()).unwrap()
            };
            let (list, list_bytes) =
                encode_type_object(&TypeObject::new(TypeForm::List { item: float }).unwrap())
                    .unwrap();
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
                ObjectKey::from_digest(ObjectDomain::Type, float.bytes()),
                float_bytes,
            );
            hostile.objects.insert(
                ObjectKey::from_digest(ObjectDomain::Type, list.bytes()),
                list_bytes,
            );
            assert_f64_source_rejected(
                &hostile,
                if malformed_tag {
                    "kernel_f64_type_tag"
                } else {
                    "kernel_type_graph_generation"
                },
            );
        }
    }
}

#[test]
fn f64_successor_checks_rebuilds_and_executes_genuine_pre_f64_source_without_rewriting_it() {
    use crate::platform::compiler::{compile_immutable, load_artifact};
    use crate::platform::execution::{
        ExecutionControl,
        normalized::{NormalizedProgram, NormalizedRunPolicy, run_graph_tests},
    };

    // These are exact pre-campaign bytes, not a predecessor-shaped object produced by the new
    // serializer. Their original source and paths are retained in graph16-standard.json.
    let bytes = include_bytes!("../../../tests/fixtures/graph16-standard.lkjp");
    let transport =
        "package_transport_a7ab2c4b6ff765b3aeae27f15f0b51445f58d61608a41ccd01b4d24b3b66cf20"
            .parse()
            .unwrap();
    let container = PackageContainer::decode(bytes, transport).unwrap();
    assert_eq!(container.encode().unwrap(), bytes);
    assert_eq!(
        container.root.package_revision.to_string(),
        "package_revision_53634416f408feddc2207ca97dc064e855d94ae6ca670f8bf39810a9eea58410"
    );
    let original_objects = container.objects.clone();
    let admitted = container.admit().unwrap();
    let independent = crate::platform::package_transport::oracle::reconstruct(&container).unwrap();
    let package = &admitted.packages[&container.root.package_revision];
    // The Graph 16 producer retained accepted Graph 15 authority for this unchanged library.
    // An executable generation never authorizes rewriting the package's older meaning identity.
    assert_eq!(package.snapshot.root.graph_contract_version, 15);
    assert_eq!(
        package.snapshot.owners,
        independent.snapshots[&package.snapshot.root.package_id].owners
    );
    assert_eq!(
        package.snapshot.types,
        independent.snapshots[&package.snapshot.root.package_id].types
    );
    validate_full(&package.snapshot).unwrap();
    assert!(package.revision.dependencies.is_empty());

    let rebuilt = compile_immutable(package, &container.objects, &[]).unwrap();
    let artifacts = [
        include_bytes!("../../../tests/fixtures/graph16-standard.lkja").as_slice(),
        rebuilt.artifact.bytes.as_slice(),
    ];
    let mut observations = Vec::new();
    for artifact in artifacts {
        let program = NormalizedProgram::prepare(load_artifact(artifact).unwrap()).unwrap();
        let receipt = run_graph_tests(
            &package.snapshot,
            &program,
            None,
            NormalizedRunPolicy::default(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
        assert!(
            receipt.passed > 0,
            "the genuine standard includes graph-owned expected-value tests"
        );
        assert_eq!(receipt.failed, 0);
        assert_eq!(receipt.differential, "equal");
        observations.push(receipt.passed);
    }
    assert_eq!(observations[0], observations[1]);
    assert_eq!(container.objects, original_objects);
    assert_eq!(container.encode().unwrap(), bytes);

    let temporary = tempfile::tempdir().unwrap();
    let target = GraphRepository::create(
        &temporary.path().join("consumer"),
        &crate::platform::kernel::tests::transport_snapshot(),
        None,
    )
    .unwrap();
    let head = target.repository.current().unwrap().head;
    target
        .repository
        .stage_package_transport(transport, bytes)
        .unwrap();
    assert_eq!(target.repository.current().unwrap().head, head);
    assert!(
        target
            .repository
            .object_store()
            .unwrap()
            .staging_leftovers()
            .is_empty()
    );
}

#[test]
fn f64_successor_admits_supported_synthetic_graph16_owner_envelopes() {
    // This bounded codec fixture deliberately emits supported owner generation 16. It is not
    // historical evidence: the authentic standard's Graph 16 producer retained a Graph 15 root.
    let (source, _) = f64_test_source(16);
    assert!(
        source
            .container
            .objects
            .values()
            .any(|bytes| bytes.starts_with(b"LKJOWN16"))
    );
    let bytes = source.container.encode().unwrap();
    let decoded = PackageContainer::decode(&bytes, source.container.root.transport).unwrap();
    assert!(decoded.admit().is_ok());
    assert!(crate::platform::package_transport::oracle::reconstruct(&decoded).is_ok());
}

#[test]
fn f64_successor_checks_stages_and_rebuilds_genuine_graph16_transaction_library() {
    use crate::platform::compiler::{LoadedArtifact, compile_immutable, load_artifact};
    use crate::platform::execution::normalized::NormalizedProgram;

    let bytes = include_bytes!("../../../tests/fixtures/graph16-transaction-library.lkjp");
    let transport =
        "package_transport_105f8fa068e50a32762f61e220086a11cf073620369c70be4baeb94fa02d5778"
            .parse()
            .unwrap();
    let container = PackageContainer::decode(bytes, transport).unwrap();
    assert_eq!(container.encode().unwrap(), bytes);
    assert_eq!(
        container.root.package_revision.to_string(),
        "package_revision_0528bf6c37becea151d02b4ce34f08f897bb76e60f69e1a9a3a203c3abafe1ad"
    );
    let admitted = container.admit().unwrap();
    let independent = crate::platform::package_transport::oracle::reconstruct(&container).unwrap();
    let root = &admitted.packages[&container.root.package_revision];
    assert_eq!(root.snapshot.root.graph_contract_version, 16);
    assert!(root.snapshot.owners.values().any(|record| matches!(record, OwnerRecord::Expression(expression) if matches!(expression.operation, ExpressionOperation::TransactionOutcome { .. }))));
    assert!(
        container
            .objects
            .values()
            .any(|bytes| bytes.starts_with(b"LKJOWN16"))
    );

    let mut rebuilt = BTreeMap::<PackageRevisionDigest, LoadedArtifact>::new();
    for revision in &admitted.dependency_order {
        let package = &admitted.packages[revision];
        validate_full(&package.snapshot).unwrap();
        assert_eq!(
            package.snapshot.owners,
            independent.snapshots[&package.snapshot.root.package_id].owners
        );
        assert_eq!(
            package.snapshot.types,
            independent.snapshots[&package.snapshot.root.package_id].types
        );
        let dependencies = package
            .revision
            .dependencies
            .iter()
            .map(|dependency| rebuilt[&dependency.package_revision].clone())
            .collect::<Vec<_>>();
        let linked = compile_immutable(package, &container.objects, &dependencies).unwrap();
        let artifact = load_artifact(&linked.artifact.bytes).unwrap();
        assert_eq!(artifact.manifest.contract_version, 21);
        assert_eq!(
            artifact.manifest.root_package,
            package.snapshot.root.package_id
        );
        NormalizedProgram::prepare(artifact.clone()).unwrap();
        rebuilt.insert(*revision, artifact);
    }
    assert!(rebuilt.contains_key(&container.root.package_revision));
    assert_eq!(container.encode().unwrap(), bytes);

    let temporary = tempfile::tempdir().unwrap();
    let target = GraphRepository::create(
        &temporary.path().join("consumer"),
        &crate::platform::kernel::tests::transport_snapshot(),
        None,
    )
    .unwrap();
    let head = target.repository.current().unwrap().head;
    target
        .repository
        .stage_package_transport(transport, bytes)
        .unwrap();
    assert_eq!(target.repository.current().unwrap().head, head);
    assert!(
        target
            .repository
            .object_store()
            .unwrap()
            .staging_leftovers()
            .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn f64_successor_executes_genuine_graph16_transaction_completion_with_fresh_data() {
    use crate::platform::compiler::load_artifact;
    use crate::platform::data::{DataLimits, DataStore};
    use crate::platform::deployment::PreparedDeployment;
    use crate::platform::execution::ExecutionControl;

    let original = include_bytes!("../../../tests/fixtures/graph16-transaction-consumer.lkja");
    let artifact = load_artifact(original).unwrap();
    assert_eq!(artifact.manifest.contract_version, 20);
    assert_eq!(artifact.manifest.graph_contract_version, 16);
    assert_eq!(
        artifact.bundle_digest.to_string(),
        "artifact_bundle_90d3efba04601441578e65a1cf33aba1591be13d1bd17377f26aba4cbe5d420a"
    );
    assert!(artifact.manifest.packages.iter().any(|package| package.package_revision.to_string() == "package_revision_0528bf6c37becea151d02b4ce34f08f897bb76e60f69e1a9a3a203c3abafe1ad"));
    let temporary = tempfile::tempdir().unwrap();
    let artifact_path = temporary.path().join("consumer.lkja");
    std::fs::write(&artifact_path, original).unwrap();
    for store in ["numbers", "texts"] {
        DataStore::initialize(&temporary.path().join(store)).unwrap();
    }
    let descriptor = serde_json::json!({
        "artifact": "consumer.lkja",
        "target": "empty-outcome",
        "listen": null,
        "http": null,
        "session": null,
        "worker": null,
        "streams": {
            "maximum_chunk_bytes": 65536,
            "maximum_buffered_chunks": 8,
            "maximum_total_bytes": 67108864,
            "maximum_live_streams": 1024
        },
        "configuration": {"suffix": {"kind": "text", "value": "!"}},
        "secrets": [],
        "grants": [
            {
                "requirement": "counters",
                "sharing_domain": "requirement-numbers",
                "authority_revision": "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1",
                "adapter": {"kind": "data", "root": "numbers", "namespace": "number-cells", "limits": DataLimits::default()}
            },
            {
                "requirement": "settings-store",
                "sharing_domain": "requirement-texts",
                "authority_revision": "a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2",
                "adapter": {"kind": "data", "root": "texts", "namespace": "text-cells", "limits": DataLimits::default()}
            },
            {
                "requirement": "suffix-config",
                "sharing_domain": "requirement-configuration",
                "authority_revision": "a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3",
                "adapter": {"kind": "configuration"}
            }
        ]
    });
    let descriptor_path = temporary.path().join("deployment.json");
    std::fs::write(&descriptor_path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let control = ExecutionControl::uncancelled();
    let (deployment, invocation) = PreparedDeployment::load_foreground(
        &descriptor_path,
        b"[]",
        tokio::runtime::Handle::current(),
        &control,
    )
    .unwrap();
    let receipt = deployment
        .run_foreground(invocation, std::future::pending())
        .await
        .unwrap();
    // Fixed original command 669 expected result. A completed empty transaction has a real
    // Committed(Unit) result without inventing an application write or reinterpreting false.
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&receipt.result_json).unwrap(),
        serde_json::json!({"case": "Committed", "value": null})
    );
    assert_eq!(receipt.production.capability_calls, 1);
    assert_eq!(receipt.production.live_transactions_after, 0);
    assert_eq!(receipt.shutdown.remaining_tasks, 0);
    assert!(receipt.shutdown.cleanup_failures.is_empty());
    for (root, namespace) in [("numbers", "number-cells"), ("texts", "text-cells")] {
        let store = DataStore::open(
            &temporary.path().join(root),
            namespace,
            DataLimits::default(),
        )
        .unwrap();
        let checked = store.verify().unwrap();
        assert_eq!(checked.records, 0);
        assert_eq!(checked.staging_leftovers, 0);
    }
    assert_eq!(std::fs::read(&artifact_path).unwrap(), original);
}
