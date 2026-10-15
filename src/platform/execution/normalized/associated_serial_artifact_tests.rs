//! Genuine public publication/import, strict artifact admission and serial lending.
//! Immutable source inputs are shared; changed bytes always require fresh admission.

use super::*;
use crate::platform::builtin_standard::BuiltinStandard;
use crate::platform::compiler::CompilationPayload;
use crate::platform::control::decode_compact_change;
use crate::platform::execution::normalized::run_pure_command;
use crate::platform::normalized_lifecycle::PackageReference;
use crate::platform::publication::ExportedPackageTransport;
use crate::platform::semantic_id::AssociatedTypeMemberId;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

const CONSUMER: &str = include_str!("associated_serial_fixture/consumer.lkjc");
const SUPPLIER: &str = include_str!("associated_serial_fixture/chunked.lkjc");
const APPLICATION: &str = include_str!("associated_serial_fixture/application.lkjc");
const BLOCK: &str = "assoc_type_c7000000000000000000000000000001";
const SPARE: &str = "assoc_type_c7000000000000000000000000000002";
const ARGUMENTS: &[u8] = b"[[17,-2,9]]";
const EXPECTED: &[u8] = br#"{"first":3,"second":3}"#;

static FIXTURE_BUILDS: AtomicU64 = AtomicU64::new(0);

fn fixture_key(sources: [&str; 3], transport: &[u8], artifact: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.associated-serial-test-fixture");
    hasher.update(&crate::platform::canonical_schema::CANONICAL_SCHEMA_HASH);
    hasher.update(&crate::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH);
    for part in [
        "DeterministicBaseline",
        "associated-serial-consumer",
        "associated-serial-supplier",
        "associated-serial-application",
    ]
    .into_iter()
    .chain(sources)
    {
        hasher.update(&(part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    for bytes in [transport, artifact] {
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    *hasher.finalize().as_bytes()
}

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
    pub source: crate::platform::kernel::KernelSnapshot,
    pub artifact: Arc<[u8]>,
    pub loaded: crate::platform::compiler::LoadedArtifact,
    reference: Arc<PackageReference>,
    binding: super::super::reference::NormalizedReferenceBinding,
}

pub(in crate::platform) fn fixture() -> Arc<SerialArtifactFixture> {
    static SHARED: OnceLock<([u8; 32], Arc<SerialArtifactFixture>)> = OnceLock::new();
    let standard = BuiltinStandard::load().unwrap();
    let key = fixture_key(
        [CONSUMER, SUPPLIER, APPLICATION],
        standard.transport_bytes(),
        standard.artifact_bytes(),
    );
    let (bound, fixture) = SHARED.get_or_init(|| (key, Arc::new(build_fixture(standard))));
    assert_eq!(*bound, key);
    Arc::clone(fixture)
}

fn build_fixture(standard: &BuiltinStandard) -> SerialArtifactFixture {
    FIXTURE_BUILDS.fetch_add(1, Ordering::Relaxed);
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
    let loaded = load_artifact(&prepared.artifact_bytes).unwrap();
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
    let binding = prepared.reference.binding().unwrap();
    assert_eq!(binding.revision, Some(accepted));
    assert_eq!(application.view_current().unwrap().revision(), accepted);
    let fixture = SerialArtifactFixture {
        source,
        artifact: Arc::from(prepared.artifact_bytes),
        loaded,
        reference: Arc::new(prepared.reference),
        binding,
    };
    temporary.close().unwrap();
    fixture
}

#[test]
fn associated_serial_public_imported_chunked_descendant_loads_and_executes_both_engines() {
    let fixture = fixture();
    let original_hash = blake3::hash(&fixture.artifact);
    let temporary = tempfile::tempdir().unwrap();
    let copy_path = temporary.path().join("serial-copy.lkja");
    std::fs::write(&copy_path, &fixture.artifact).unwrap();
    let copied = std::fs::read(&copy_path).unwrap();
    assert_eq!(copied.as_slice(), fixture.artifact.as_ref());
    assert_eq!(blake3::hash(&copied), original_hash);
    let loaded = load_artifact(&copied).unwrap();
    loaded.require_current_admission(|| Ok(())).unwrap();
    assert_eq!(loaded.bundle_digest, fixture.loaded.bundle_digest);
    let fresh_program = Arc::new(NormalizedProgram::prepare(loaded).unwrap());
    temporary.close().unwrap();
    assert!(!copy_path.exists());
    let storage = super::super::owned_storage::StorageObservation::start();
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    let rejected = run_pure_command(
        fixture.reference.as_ref(),
        &fresh_program,
        &Name::new("associated-serial").unwrap(),
        ARGUMENTS,
        NormalizedCommandPolicy::default(),
        &cancelled,
    );
    assert_eq!(
        rejected.unwrap_err().class,
        crate::platform::diagnostic::DiagnosticClass::Cancelled
    );
    assert_eq!(storage.live(), (0, 0));
    let receipt = run_pure_command(
        fixture.reference.as_ref(),
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
    assert_eq!(blake3::hash(&fixture.artifact), original_hash);
    assert_eq!(fixture.reference.binding().unwrap(), fixture.binding);
}

#[test]
fn associated_serial_fixture_cache_binds_actual_standard_bytes_and_isolates_fault_clones() {
    let standard = BuiltinStandard::load().unwrap();
    let sources = [CONSUMER, SUPPLIER, APPLICATION];
    let key = fixture_key(
        sources,
        standard.transport_bytes(),
        standard.artifact_bytes(),
    );
    for index in 0..sources.len() {
        let changed = format!("{}\n", sources[index]);
        let mut altered = sources;
        altered[index] = &changed;
        assert_ne!(
            fixture_key(
                altered,
                standard.transport_bytes(),
                standard.artifact_bytes()
            ),
            key
        );
    }
    let mut changed_transport = standard.transport_bytes().to_vec();
    let last = changed_transport.last_mut().unwrap();
    *last ^= 1;
    assert_ne!(
        fixture_key(sources, &changed_transport, standard.artifact_bytes()),
        key
    );
    let mut changed_artifact = standard.artifact_bytes().to_vec();
    let last = changed_artifact.last_mut().unwrap();
    *last ^= 1;
    assert_ne!(
        fixture_key(sources, standard.transport_bytes(), &changed_artifact),
        key
    );
    let fixture = fixture();
    let same = self::fixture();
    assert_eq!(FIXTURE_BUILDS.load(Ordering::Relaxed), 1);
    assert!(Arc::ptr_eq(&fixture, &same));
    let original_name = fixture.source.root.package_name.clone();
    let mut source = fixture.source.clone();
    source.root.package_name = Name::new("private-fault-clone").unwrap();
    assert_eq!(fixture.source.root.package_name, original_name);
    assert_ne!(source.root.package_name, original_name);
    let mut corrupted = fixture.loaded.clone();
    let first = *corrupted.objects.keys().next().unwrap();
    corrupted.objects.remove(&first).unwrap();
    assert!(corrupted.require_current_admission(|| Ok(())).is_err());
    fixture.loaded.require_current_admission(|| Ok(())).unwrap();
}
