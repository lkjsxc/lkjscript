//! Genuine public publication/import, strict artifact admission and serial lending.
//! This module is proposed/unrun until the coordinated admission guards are opened.

use super::*;
use crate::platform::builtin_standard::BuiltinStandard;
use crate::platform::compiler::CompilationPayload;
use crate::platform::control::decode_compact_change;
use crate::platform::execution::normalized::run_pure_command;
use crate::platform::publication::ExportedPackageTransport;
use crate::platform::semantic_id::AssociatedTypeMemberId;
use std::collections::BTreeSet;
use std::sync::Arc;

const CONSUMER: &str = include_str!("associated_serial_fixture/consumer.lkjc");
const SUPPLIER: &str = include_str!("associated_serial_fixture/chunked.lkjc");
const APPLICATION: &str = include_str!("associated_serial_fixture/application.lkjc");
const BLOCK: &str = "assoc_type_c7000000000000000000000000000001";
const SPARE: &str = "assoc_type_c7000000000000000000000000000002";
const ARGUMENTS: &[u8] = b"[[17,-2,9]]";
const EXPECTED: &[u8] = br#"{"first":3,"second":3}"#;

fn publish_native(repository: &GraphRepository, selections: &str, literal: &str) {
    let before = repository.view_current().unwrap().revision();
    let request = format!("request base={before}\n{selections}{literal}");
    let decoded = decode_compact_change("associated-serial.lkjc", request.as_bytes()).unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert_eq!(repository.view_current().unwrap().revision(), before);
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert_ne!(repository.view_current().unwrap().revision(), before);
    // Publication's incremental validation must agree with the real full oracle.
    // This reads accepted bytes and runs the complete validator; no gate is bypassed.
    let snapshot = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap();
    crate::platform::kernel::validate_full(&snapshot.value).unwrap();
}

fn stage(repository: &GraphRepository, exported: &ExportedPackageTransport) {
    let before = repository.view_current().unwrap().revision();
    let original = exported.container.clone();
    repository
        .stage_package_transport(exported.transport_digest, &exported.container)
        .unwrap();
    assert_eq!(repository.view_current().unwrap().revision(), before);
    assert_eq!(exported.container, original);
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

pub(in crate::platform) struct SerialArtifactFixture {
    pub temporary: tempfile::TempDir,
    pub prepared: crate::platform::normalized_lifecycle::PreparedApplication,
    pub source: crate::platform::kernel::KernelSnapshot,
    pub artifact: Vec<u8>,
    pub loaded: crate::platform::compiler::LoadedArtifact,
}

pub(in crate::platform) fn fixture() -> SerialArtifactFixture {
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
    let consumer = create("consumer", b"associated-serial-consumer");
    publish_native(&consumer, "", CONSUMER);
    let consumer_export = consumer.export_package_transport().unwrap();

    let supplier = create("supplier", b"associated-serial-supplier");
    stage(&supplier, &consumer_export);
    let standard = BuiltinStandard::load().unwrap();
    let before_stage = supplier.view_current().unwrap().revision();
    supplier
        .stage_package_transport(standard.package_transport, standard.transport_bytes())
        .unwrap();
    assert_eq!(supplier.view_current().unwrap().revision(), before_stage);
    let supplier_selections = format!(
        "{}add.dependency package={} semantic-revision={} package-revision={}\n",
        selection("blockread-consumer", &consumer_export),
        standard.package,
        standard.semantic_revision,
        standard.package_revision,
    );
    publish_native(&supplier, &supplier_selections, SUPPLIER);
    let supplier_export = supplier.export_package_transport().unwrap();

    let application = create("application", b"associated-serial-application");
    stage(&application, &consumer_export);
    stage(&application, &supplier_export);
    let application_selections = format!(
        "{}{}",
        selection("blockread-consumer", &consumer_export),
        selection("blockread-chunked", &supplier_export),
    );
    publish_native(&application, &application_selections, APPLICATION);
    let application_export = application.export_package_transport().unwrap();
    let accepted = application.view_current().unwrap().revision();

    // The production lifecycle reconstructs complete independent canonical source,
    // not a private schema core or a single root snapshot without dependency bodies.
    let prepared =
        crate::platform::normalized_lifecycle::prepare_repository(application.clone()).unwrap();
    assert_eq!(prepared.revision, accepted);
    assert_eq!(application.view_current().unwrap().revision(), accepted);
    let artifact = prepared.artifact_bytes.clone();
    let copy_path = temporary.path().join("serial-copy.lkja");
    std::fs::write(&copy_path, &artifact).unwrap();
    let copied = std::fs::read(&copy_path).unwrap();
    assert_eq!(copied, artifact);
    let loaded = load_artifact(&copied).unwrap();
    loaded.require_current_admission(|| Ok(())).unwrap();
    assert_eq!(loaded.bundle_digest, prepared.artifact_bundle);
    let expected_packages = BTreeSet::from([
        application_export.revision.package,
        consumer_export.revision.package,
        supplier_export.revision.package,
        standard.package,
    ]);
    assert_eq!(
        loaded
            .manifest
            .packages
            .iter()
            .map(|package| package.package)
            .collect::<BTreeSet<_>>(),
        expected_packages,
    );
    for exported in [&application_export, &consumer_export, &supplier_export] {
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

    // Both members were authored before publication. Spare is genuinely unused;
    // it is neither appended to an admitted catalogue nor used as the result oracle.
    let block: AssociatedTypeMemberId = BLOCK.parse().unwrap();
    let spare: AssociatedTypeMemberId = SPARE.parse().unwrap();
    let mut contract_found = false;
    let mut implementation_found = false;
    for (key, bytes) in &loaded.objects {
        if key.domain != crate::platform::storage::object::ObjectDomain::CompilerUnit {
            continue;
        }
        let unit = crate::platform::compiler::CompilationUnit::decode(bytes, *key).unwrap();
        match &unit.payload {
            CompilationPayload::OwnedContract(contract)
                if unit.source.package == consumer_export.revision.package =>
            {
                assert_eq!(
                    contract
                        .associated_types
                        .iter()
                        .map(|member| member.id)
                        .collect::<Vec<_>>(),
                    vec![block, spare],
                );
                contract_found = true;
            }
            CompilationPayload::OwnedImplementation(implementation)
                if unit.source.package == supplier_export.revision.package =>
            {
                assert_eq!(
                    implementation
                        .associated_types
                        .iter()
                        .map(|binding| binding.member)
                        .collect::<BTreeSet<_>>(),
                    BTreeSet::from([block, spare]),
                );
                implementation_found = true;
            }
            _ => {}
        }
    }
    assert!(contract_found && implementation_found);

    let source = application
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    SerialArtifactFixture {
        temporary,
        prepared,
        source,
        artifact,
        loaded,
    }
}

#[test]
fn associated_serial_public_imported_chunked_descendant_loads_and_executes_both_engines() {
    let fixture = fixture();
    let original_hash = blake3::hash(&fixture.artifact);
    let copy_path = fixture.temporary.path().join("serial-copy.lkja");
    let fresh_program = Arc::new(NormalizedProgram::prepare(fixture.loaded.clone()).unwrap());
    let storage = super::super::owned_storage::StorageObservation::start();
    let receipt = run_pure_command(
        &fixture.prepared.reference,
        &fresh_program,
        &Name::new("associated-serial").unwrap(),
        ARGUMENTS,
        NormalizedCommandPolicy::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(receipt.result_json.as_slice(), EXPECTED);
    assert_eq!(receipt.differential, "equal");
    assert!(receipt.reference.canonical_owner_reads > 0);
    assert_eq!(receipt.production.parallel_scopes, 0);
    assert_eq!(receipt.production.capability_calls, 0);
    assert_eq!(receipt.reference.capability_calls, 0);
    assert_eq!(
        (
            receipt.production.live_call_frames_after,
            receipt.production.live_locals_after,
            receipt.production.live_type_bindings_after,
            receipt.production.live_operands_after,
            receipt.production.live_transactions_after,
            receipt.production.live_handles_after,
        ),
        (0, 0, 0, 0, 0, 0),
    );
    assert_eq!(
        (
            receipt.reference.live_call_frames_after,
            receipt.reference.live_control_frames_after,
            receipt.reference.live_local_scopes_after,
            receipt.reference.live_type_scopes_after,
            receipt.reference.live_effect_scopes_after,
            receipt.reference.live_allowances_after,
            receipt.reference.live_transactions_after,
            receipt.reference.live_handles_after,
        ),
        (0, 0, 0, 0, 0, 0, 0, 0),
    );
    assert!(storage.created() > 0);
    assert_eq!(storage.live(), (0, 0));
    storage.assert_owners_released_after_loans();
    assert_eq!(std::fs::read(&copy_path).unwrap(), fixture.artifact);
    assert_eq!(
        blake3::hash(&std::fs::read(&copy_path).unwrap()),
        original_hash
    );
    assert_eq!(
        fixture
            .prepared
            .repository
            .view_current()
            .unwrap()
            .revision(),
        fixture.prepared.revision
    );
}
