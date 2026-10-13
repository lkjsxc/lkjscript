//! Native separate-package read views through the complete preparation lifecycle.
//! The public literals and expected cases are shared with the copied-CLI witnesses.

use super::*;
use crate::platform::builtin_standard::BuiltinStandard;
use crate::platform::control::decode_compact_change_in_repository;
use crate::platform::execution::normalized::{NormalizedRunObservation, run_pure_command};
use crate::platform::publication::ExportedPackageTransport;

const CONSUMER: &str =
    include_str!("../../../../tests/public_cli/read_view_fixtures/01-consumer.lkjc");
const TRANSFORMER: &str =
    include_str!("../../../../tests/public_cli/read_view_fixtures/02-transformer.lkjc");
const FLAT: &str = include_str!("../../../../tests/public_cli/read_view_fixtures/03-flat.lkjc");
const FLAT_JOINED: &str =
    include_str!("../../../../tests/public_cli/read_view_fixtures/03-flat-joined.lkjc");
const CHUNKED: &str =
    include_str!("../../../../tests/public_cli/read_view_fixtures/04-chunked.lkjc");
const CHUNKED_JOINED: &str =
    include_str!("../../../../tests/public_cli/read_view_fixtures/04-chunked-joined.lkjc");
const APPLICATION: &str =
    include_str!("../../../../tests/public_cli/read_view_fixtures/05-application.lkjc");
const APPLICATION_JOINED: &str =
    include_str!("../../../../tests/public_cli/read_view_fixtures/05-application-joined.lkjc");
const CASES: &str = include_str!("../../../../tests/public_cli/read_view_fixtures/cases.json");

fn publish_native(repository: &GraphRepository, selections: &str, literal: &str) {
    let before = repository.view_current().unwrap().revision();
    let request = format!("request base={before}\n{selections}{literal}");
    let decoded = decode_compact_change_in_repository(
        "associated-read-view.lkjc",
        request.as_bytes(),
        repository,
    )
    .unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert_eq!(repository.view_current().unwrap().revision(), before);
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert_ne!(repository.view_current().unwrap().revision(), before);
    let source = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap();
    crate::platform::kernel::validate_full(&source.value).unwrap();
}

fn stage(repository: &GraphRepository, exported: &ExportedPackageTransport) {
    let before = repository.view_current().unwrap().revision();
    repository
        .stage_package_transport(exported.transport_digest, &exported.container)
        .unwrap();
    assert_eq!(repository.view_current().unwrap().revision(), before);
}

fn selection(alias: &str, exported: &ExportedPackageTransport) -> String {
    format!(
        "add.dependency package={} semantic-revision={} package-revision={}\n\
         declarations.begin\n(units (use {alias} {} {}))\ndeclarations.end\n",
        exported.revision.package,
        exported.revision.revision.revision_id().unwrap(),
        exported.revision_digest,
        exported.revision.package,
        exported.revision_digest,
    )
}

fn extend_native(repository: &GraphRepository, name: &str, literal: &str, selections: &str) {
    let source = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap();
    let modules = source
        .value
        .owners
        .iter()
        .filter_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Module(id), OwnerRecord::Module(module)) if module.name.as_str() == name => {
                Some(*id)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(modules.len(), 1);
    assert_eq!(literal.matches("MODULE_OWNER").count(), 1);
    publish_native(
        repository,
        selections,
        &literal.replace("MODULE_OWNER", &modules[0].to_string()),
    );
}

#[test]
fn associated_read_view_public_imported_closure_prepares_and_executes_both_engines() {
    let temporary = tempfile::tempdir().unwrap();
    let create = |directory: &str, seed: &[u8]| {
        GraphRepository::create(
            &temporary.path().join(directory),
            &empty_normalized_snapshot(seed),
            None,
        )
        .unwrap()
        .repository
    };
    let standard = BuiltinStandard::load().unwrap();
    let standard_selection = format!(
        "add.dependency package={} semantic-revision={} package-revision={}\n",
        standard.package, standard.semantic_revision, standard.package_revision,
    );
    let stage_standard = |repository: &GraphRepository| {
        let before = repository.view_current().unwrap().revision();
        repository
            .stage_package_transport(standard.package_transport, standard.transport_bytes())
            .unwrap();
        assert_eq!(repository.view_current().unwrap().revision(), before);
    };

    // The generic contract and reducer exist before either representation supplier.
    let consumer = create("consumer", b"read-view-lifecycle-consumer");
    stage_standard(&consumer);
    publish_native(&consumer, &standard_selection, CONSUMER);
    let consumer_export = consumer.export_package_transport().unwrap();
    let transformer = create("transformer", b"read-view-lifecycle-transformer");
    stage(&transformer, &consumer_export);
    publish_native(
        &transformer,
        &selection("readview-consumer", &consumer_export),
        TRANSFORMER,
    );
    let transformer_export = transformer.export_package_transport().unwrap();

    let flat = create("flat", b"read-view-lifecycle-flat");
    let chunked = create("chunked", b"read-view-lifecycle-chunked");
    for (repository, literal, module, joined) in [
        (&flat, FLAT, "readview-flat", FLAT_JOINED),
        (&chunked, CHUNKED, "readview-chunked", CHUNKED_JOINED),
    ] {
        stage_standard(repository);
        stage(repository, &consumer_export);
        publish_native(
            repository,
            &format!(
                "{standard_selection}{}",
                selection("readview-consumer", &consumer_export),
            ),
            literal,
        );
        // Unselected joined syntax is retained in the same closure as the public case.
        extend_native(repository, module, joined, "");
    }
    let flat_export = flat.export_package_transport().unwrap();
    let chunked_export = chunked.export_package_transport().unwrap();

    let application = create("application", b"read-view-lifecycle-application");
    let exports = [
        ("readview-consumer", &consumer_export),
        ("readview-transformer", &transformer_export),
        ("readview-flat", &flat_export),
        ("readview-chunked", &chunked_export),
    ];
    for (_, exported) in exports {
        stage(&application, exported);
    }
    let selections = exports
        .iter()
        .map(|(alias, exported)| selection(alias, exported))
        .collect::<String>();
    publish_native(&application, &selections, APPLICATION);
    // Namespace aliases are selected per authored request. Reuse the same exact
    // dependencies; the joined extension adds no dependency inventory entries.
    let joined_selections = exports
        .iter()
        .map(|(alias, exported)| {
            format!(
                "declarations.begin\n(units (use {alias} {} {}))\ndeclarations.end\n",
                exported.revision.package, exported.revision_digest,
            )
        })
        .collect::<String>();
    extend_native(
        &application,
        "readview-app",
        APPLICATION_JOINED,
        &joined_selections,
    );
    let application_export = application.export_package_transport().unwrap();
    let accepted = application.view_current().unwrap().revision();

    // Production performs complete canonical reference reconstruction, dependency
    // compilation, strict artifact admission and concrete prepared-type closure.
    let prepared =
        crate::platform::normalized_lifecycle::prepare_repository(application.clone()).unwrap();
    assert_eq!(prepared.revision, accepted);
    assert_eq!(application.view_current().unwrap().revision(), accepted);
    let copy = temporary.path().join("read-view-copy.lkja");
    std::fs::write(&copy, &prepared.artifact_bytes).unwrap();
    let copied = std::fs::read(&copy).unwrap();
    assert_eq!(copied, prepared.artifact_bytes);
    let loaded = load_artifact(&copied).unwrap();
    loaded.require_current_admission(|| Ok(())).unwrap();
    assert_eq!(loaded.bundle_digest, prepared.artifact_bundle);
    assert_eq!(
        loaded
            .manifest
            .packages
            .iter()
            .map(|package| package.package)
            .collect::<BTreeSet<_>>(),
        exports
            .iter()
            .map(|(_, exported)| exported.revision.package)
            .chain([application_export.revision.package, standard.package])
            .collect::<BTreeSet<_>>(),
    );
    for exported in exports
        .iter()
        .map(|(_, exported)| *exported)
        .chain([&application_export])
    {
        let package = loaded.package(exported.revision.package).unwrap();
        assert_eq!(package.package_revision, exported.revision_digest);
        assert_eq!(
            package.semantic_revision,
            exported.revision.revision.revision_id().unwrap(),
        );
    }
    let builtin = loaded.package(standard.package).unwrap();
    assert_eq!(builtin.package_revision, standard.package_revision);
    assert_eq!(builtin.semantic_revision, standard.semantic_revision);
    let fresh_program = Arc::new(NormalizedProgram::prepare(loaded).unwrap());

    // Locate a pre-dispatch allocation refusal for real joined view sharing from
    // observed execution, rather than deriving an allowance from object sizes.
    // This is a closed Task VM invocation, not a pure-project differential route.
    let task = Name::new("readview-joined-flat").unwrap();
    let joined_cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/public_cli/read_view_fixtures/joined-cases.json"
    ))
    .unwrap();
    let first_case: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let arguments = serde_json::to_vec(&first_case[0]["arguments"]).unwrap();
    let expected = serde_json::to_vec(&joined_cases[0]["range"]).unwrap();
    let probe = |maximum_allocated_bytes| {
        let storage = super::super::owned_storage::StorageObservation::start();
        let control = ExecutionControl::uncancelled();
        let invocation = super::super::runner::prepare_command_invocation(
            &fresh_program,
            &task,
            &arguments,
            crate::platform::json::JsonLimits::default(),
            &control,
        )
        .unwrap();
        assert!(invocation.task);
        let observer = std::sync::Mutex::new(None);
        let result = NormalizedVm::for_test(
            &fresh_program,
            NormalizedRunPolicy {
                maximum_allocated_bytes,
                ..NormalizedRunPolicy::foreground()
            },
        )
        .observing_checked(&observer)
        .invoke_root_target(&task, invocation.arguments, None, &control);
        let observation = observer.lock().unwrap().take().unwrap();
        match &result {
            Ok((value, _)) => {
                assert_eq!(
                    super::super::codec::encode_typed_with_control(
                        fresh_program.as_ref(),
                        value,
                        invocation.result_type,
                        crate::platform::json::JsonLimits::default(),
                        &control,
                    )
                    .unwrap(),
                    expected,
                );
            }
            Err(error) => {
                assert_eq!(
                    error.class,
                    crate::platform::execution::ExecutionFailureClass::Resource
                );
                assert!(error.code.contains("allocation"));
            }
        }
        let success = result.is_ok();
        drop(result);
        if observation.parallel_scopes > 0 {
            assert!(storage.created() > 0);
        }
        assert_eq!(storage.live(), (0, 0));
        storage.assert_owners_released_after_loans();
        assert_eq!(observation.live_call_frames_after, 0);
        assert_eq!(observation.live_locals_after, 0);
        assert_eq!(observation.live_type_bindings_after, 0);
        assert_eq!(observation.live_operands_after, 0);
        assert_eq!(observation.live_transactions_after, 0);
        assert_eq!(observation.live_handles_after, 0);
        (success, observation)
    };
    let (success, baseline) = probe(None);
    assert!(success);
    let started = |observation: &NormalizedRunObservation| {
        observation.parallel_worker_dispatches + observation.parallel_inline_fallbacks > 0
    };
    assert!(started(&baseline));
    let mut low = 1;
    let mut high = baseline.allocated_bytes;
    assert!(high > low);
    let (_, first) = probe(Some(low));
    assert!(!started(&first));
    while high - low > 1 {
        let middle = low.midpoint(high);
        let (_, observation) = probe(Some(middle));
        if started(&observation) {
            high = middle;
        } else {
            low = middle;
        }
    }
    let (success, refused) = probe(Some(low));
    assert!(!success);
    assert_eq!(refused.parallel_scopes, 1);
    assert!(!started(&refused));
    let (success, recovery) = probe(Some(baseline.allocated_bytes));
    assert!(success);
    assert!(started(&recovery));

    // Literal expectations are independent of the source/compiler tables and both
    // evaluators. Each command reaches both Flat and Chunked concrete witnesses.
    let cases: serde_json::Value = serde_json::from_str(CASES).unwrap();
    for index in [0, 4] {
        let case = &cases[index];
        let expected = serde_json::json!({
            "flat": case["expected"].clone(),
            "chunked": case["expected"].clone(),
        });
        let arguments = serde_json::to_vec(&case["arguments"]).unwrap();
        let storage = super::super::owned_storage::StorageObservation::start();
        let receipt = run_pure_command(
            &prepared.reference,
            &fresh_program,
            &Name::new("readview").unwrap(),
            &arguments,
            NormalizedCommandPolicy::default(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&receipt.result_json).unwrap(),
            expected,
            "literal case {}",
            case["name"],
        );
        assert_eq!(receipt.differential, "equal");
        assert_eq!(receipt.revision, Some(accepted));
        assert!(receipt.reference.canonical_owner_reads > 0);
        assert_eq!(receipt.production.capability_calls, 0);
        assert_eq!(receipt.reference.capability_calls, 0);
        assert_eq!(receipt.production.live_call_frames_after, 0);
        assert_eq!(receipt.production.live_locals_after, 0);
        assert_eq!(receipt.production.live_type_bindings_after, 0);
        assert_eq!(receipt.production.live_operands_after, 0);
        assert_eq!(receipt.production.live_transactions_after, 0);
        assert_eq!(receipt.production.live_handles_after, 0);
        assert_eq!(receipt.reference.live_call_frames_after, 0);
        assert_eq!(receipt.reference.live_control_frames_after, 0);
        assert_eq!(receipt.reference.live_local_scopes_after, 0);
        assert_eq!(receipt.reference.live_type_scopes_after, 0);
        assert_eq!(receipt.reference.live_effect_scopes_after, 0);
        assert_eq!(receipt.reference.live_allowances_after, 0);
        assert_eq!(receipt.reference.live_transactions_after, 0);
        assert_eq!(receipt.reference.live_handles_after, 0);
        assert!(storage.created() > 0);
        assert_eq!(storage.live(), (0, 0));
        storage.assert_owners_released_after_loans();
    }
    // Passive descriptors still protect their complete source until their handle
    // is released. Use the public literal scalar oracle, including an empty owner.
    for index in [0, 4] {
        let case = &cases[index];
        let arguments = serde_json::to_vec(&case["arguments"]).unwrap();
        for target in ["readview-metadata", "readview-zero"] {
            let storage = super::super::owned_storage::StorageObservation::start();
            let receipt = run_pure_command(
                &prepared.reference,
                &fresh_program,
                &Name::new(target).unwrap(),
                &arguments,
                NormalizedCommandPolicy::default(),
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&receipt.result_json).unwrap(),
                case["expected"]["source_length"],
                "passive target {target}, literal case {}",
                case["name"],
            );
            assert_eq!(receipt.differential, "equal");
            assert_eq!(receipt.revision, Some(accepted));
            assert!(receipt.reference.canonical_owner_reads > 0);
            assert_eq!(receipt.production.capability_calls, 0);
            assert_eq!(receipt.reference.capability_calls, 0);
            assert_eq!(receipt.production.live_call_frames_after, 0);
            assert_eq!(receipt.production.live_locals_after, 0);
            assert_eq!(receipt.production.live_type_bindings_after, 0);
            assert_eq!(receipt.production.live_operands_after, 0);
            assert_eq!(receipt.production.live_transactions_after, 0);
            assert_eq!(receipt.production.live_handles_after, 0);
            assert_eq!(receipt.reference.live_call_frames_after, 0);
            assert_eq!(receipt.reference.live_control_frames_after, 0);
            assert_eq!(receipt.reference.live_local_scopes_after, 0);
            assert_eq!(receipt.reference.live_type_scopes_after, 0);
            assert_eq!(receipt.reference.live_effect_scopes_after, 0);
            assert_eq!(receipt.reference.live_allowances_after, 0);
            assert_eq!(receipt.reference.live_transactions_after, 0);
            assert_eq!(receipt.reference.live_handles_after, 0);
            assert!(storage.created() > 0);
            assert_eq!(storage.live(), (0, 0));
            storage.assert_owners_released_after_loans();
        }
    }
    assert_eq!(std::fs::read(&copy).unwrap(), copied);
    assert_eq!(application.view_current().unwrap().revision(), accepted);
}
