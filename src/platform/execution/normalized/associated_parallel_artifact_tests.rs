//! Genuine native package publication/import and strict-loaded associated Parallel.
//! Expected values are literal; the canonical reference is not a producer-table oracle.

use super::super::reference::NormalizedReferenceObservation;
use super::super::vm::NormalizedRunObservation;
use super::*;
use crate::platform::compiler::{CompilationPayload, CompilationUnit};
use crate::platform::control::decode_compact_change;
use crate::platform::normalized_lifecycle::PreparedApplication;
use crate::platform::publication::ExportedPackageTransport;
use crate::platform::runtime::structured::{StructuredExecutor, StructuredExecutorHandle};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CONSUMER: &str = include_str!("associated_parallel_fixture/consumer.lkjc");
const SUPPLIER: &str = include_str!("associated_parallel_fixture/supplier.lkjc");
const APPLICATION: &str = include_str!("associated_parallel_fixture/application.lkjc");
const NEGATIVE: &str = include_str!("associated_parallel_fixture/negative-unresolved-borrow.lkjc");

fn publish_native(repository: &GraphRepository, selections: &str, literal: &str) {
    let before = repository.view_current().unwrap().revision();
    let request = format!("request base={before}\n{selections}{literal}");
    let decoded = decode_compact_change("associated-parallel.lkjc", request.as_bytes()).unwrap();
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

struct Fixture {
    _temporary: tempfile::TempDir,
    prepared: PreparedApplication,
    program: Arc<NormalizedProgram>,
    copy: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
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
        // Consumer exists and is exported before either supplier or application.
        let consumer = create("consumer", b"associated-parallel-consumer");
        publish_native(&consumer, "", CONSUMER);
        let before_negative = consumer.view_current().unwrap().revision();
        let request = format!("request base={before_negative}\n{NEGATIVE}");
        let decoded = decode_compact_change("unresolved-borrow.lkjc", request.as_bytes()).unwrap();
        let errors = consumer
            .prepare_authored_change(&decoded.semantic, decoded.options)
            .unwrap_err();
        assert!(!errors.is_empty());
        for error in errors {
            assert_eq!(
                error.class,
                crate::platform::diagnostic::DiagnosticClass::Semantic
            );
            assert_eq!(error.code, "kernel_share_constraint");
        }
        assert_eq!(consumer.view_current().unwrap().revision(), before_negative);
        let consumer_export = consumer.export_package_transport().unwrap();

        let supplier = create("supplier", b"associated-parallel-supplier");
        stage(&supplier, &consumer_export);
        publish_native(
            &supplier,
            &selection("associated-parallel-consumer", &consumer_export),
            SUPPLIER,
        );
        let supplier_export = supplier.export_package_transport().unwrap();
        let application = create("application", b"associated-parallel-application");
        stage(&application, &consumer_export);
        stage(&application, &supplier_export);
        publish_native(
            &application,
            &format!(
                "{}{}",
                selection("associated-parallel-consumer", &consumer_export),
                selection("associated-parallel-supplier", &supplier_export)
            ),
            APPLICATION,
        );
        let application_export = application.export_package_transport().unwrap();
        let accepted = application.view_current().unwrap().revision();
        let prepared =
            crate::platform::normalized_lifecycle::prepare_repository(application).unwrap();
        assert_eq!(prepared.revision, accepted);
        let copy = temporary.path().join("parallel-copy.lkja");
        std::fs::write(&copy, &prepared.artifact_bytes).unwrap();
        let bytes = std::fs::read(&copy).unwrap();
        assert_eq!(bytes, prepared.artifact_bytes);
        let loaded = load_artifact(&bytes).unwrap();
        loaded.require_current_admission(|| Ok(())).unwrap();
        assert_eq!(loaded.bundle_digest, prepared.artifact_bundle);
        assert_eq!(
            loaded
                .manifest
                .packages
                .iter()
                .map(|p| p.package)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                application_export.revision.package,
                consumer_export.revision.package,
                supplier_export.revision.package
            ])
        );
        for exported in [&application_export, &consumer_export, &supplier_export] {
            let package = loaded.package(exported.revision.package).unwrap();
            assert_eq!(package.package_revision, exported.revision_digest);
            assert_eq!(
                package.semantic_revision,
                exported.revision.revision.revision_id().unwrap()
            );
        }
        let mut selves = BTreeSet::new();
        let mut outputs = BTreeSet::new();
        let mut implementations = 0;
        let mut cargo_layout = false;
        for (key, bytes) in &loaded.objects {
            if key.domain != crate::platform::storage::object::ObjectDomain::CompilerUnit {
                continue;
            }
            let unit = CompilationUnit::decode(bytes, *key).unwrap();
            if unit.source.package != supplier_export.revision.package {
                continue;
            }
            match &unit.payload {
                CompilationPayload::OwnedImplementation(implementation) => {
                    assert_eq!(implementation.associated_types.len(), 1);
                    assert_eq!(implementation.methods.len(), 2);
                    selves.insert(implementation.self_type);
                    outputs.insert(implementation.associated_types[0].ty);
                    implementations += 1;
                }
                CompilationPayload::OwnedRecord { fields, .. } => {
                    assert_eq!(fields.len(), 2);
                    cargo_layout = true;
                }
                _ => {}
            }
        }
        assert!(cargo_layout);
        assert_eq!(implementations, 2);
        assert_eq!(selves.len(), 1, "two exact selections share the same Self");
        assert_eq!(
            outputs.len(),
            2,
            "associated meaning must retain selected witness identity"
        );
        let program = Arc::new(NormalizedProgram::prepare(loaded).unwrap());
        Self {
            _temporary: temporary,
            prepared,
            program,
            copy,
        }
    }
    fn unchanged(&self) {
        assert_eq!(
            std::fs::read(&self.copy).unwrap(),
            self.prepared.artifact_bytes
        );
        assert_eq!(
            self.prepared.repository.view_current().unwrap().revision(),
            self.prepared.revision
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum Engine {
    Vm,
    Reference,
}
#[derive(Clone, Copy)]
struct Request {
    mirrored: bool,
    read_phase: bool,
    left_fail: bool,
    right_fail: bool,
    rounds: i64,
}
impl Request {
    fn healthy(mirrored: bool) -> Self {
        Self {
            mirrored,
            read_phase: false,
            left_fail: false,
            right_fail: false,
            rounds: 0,
        }
    }
    fn arguments(self) -> Vec<NormalizedValue> {
        vec![
            NormalizedValue::I64(-257),
            NormalizedValue::Bool(self.mirrored),
            NormalizedValue::Bool(self.read_phase),
            NormalizedValue::Bool(self.left_fail),
            NormalizedValue::Bool(self.right_fail),
            NormalizedValue::I64(self.rounds),
        ]
    }
}

enum Observation {
    Vm(NormalizedRunObservation),
    Reference(NormalizedReferenceObservation),
}
impl Observation {
    fn clean(&self) {
        match self {
            Self::Vm(o) => {
                assert_eq!(
                    (
                        o.live_call_frames_after,
                        o.live_locals_after,
                        o.live_type_bindings_after,
                        o.live_operands_after,
                        o.live_transactions_after,
                        o.live_handles_after
                    ),
                    (0, 0, 0, 0, 0, 0)
                );
                assert_eq!(o.capability_calls, 0);
            }
            Self::Reference(o) => {
                assert_eq!(
                    (
                        o.live_call_frames_after,
                        o.live_control_frames_after,
                        o.live_local_scopes_after,
                        o.live_type_scopes_after,
                        o.live_effect_scopes_after,
                        o.live_allowances_after,
                        o.live_transactions_after,
                        o.live_handles_after
                    ),
                    (0, 0, 0, 0, 0, 0, 0, 0)
                );
                assert_eq!(o.capability_calls, 0);
            }
        }
    }
    fn totals(&self) -> [u64; 3] {
        match self {
            Self::Vm(o) => [o.instructions, o.allocated_bytes, o.collection_items],
            Self::Reference(o) => [o.expressions, o.allocated_bytes, o.collection_items],
        }
    }
    fn external_calls(&self) -> u64 {
        match self {
            Self::Vm(o) => o.external_calls,
            Self::Reference(o) => o.external_calls,
        }
    }
}

struct Storage {
    cells: super::super::owned_i64_cell::StorageObservation,
    buffers: super::super::byte_buffer::StorageObservation,
    composites: super::super::owned_storage::StorageObservation,
}
impl Storage {
    fn start() -> Self {
        Self {
            cells: super::super::owned_i64_cell::StorageObservation::start(),
            buffers: super::super::byte_buffer::StorageObservation::start(),
            composites: super::super::owned_storage::StorageObservation::start(),
        }
    }
    fn clean(&self) {
        assert_eq!(self.cells.live(), (0, 0));
        assert_eq!(self.buffers.live(), (0, 0));
        assert_eq!(self.composites.live(), (0, 0));
        self.composites.assert_owners_released_after_loans();
    }
    fn parents_created(&self) {
        assert_eq!(self.cells.created(), 2);
        assert_eq!(self.buffers.created(), 2);
        assert!(self.composites.created() >= 2);
    }
    fn transferred(&self) {
        self.parents_created();
        self.composites.assert_transfers_preserve_allocations();
    }
}

fn run(
    fixture: &Fixture,
    engine: Engine,
    executor: &StructuredExecutorHandle,
    request: Request,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
) -> (Result<NormalizedValue, ExecutionError>, Option<Observation>) {
    let target = Name::new("associated-parallel").unwrap();
    match engine {
        Engine::Vm => {
            let observer = Mutex::new(None);
            let result = NormalizedVm::new(&fixture.program, policy, executor)
                .observing_checked(&observer)
                .invoke_root_target(&target, request.arguments(), None, control)
                .map(|(value, _)| value);
            let observation = observer.into_inner().unwrap().map(Observation::Vm);
            (result, observation)
        }
        Engine::Reference => {
            let observer = Mutex::new(None);
            let result = NormalizedReferenceInterpreter::from_reader(
                &fixture.prepared.reference,
                &fixture.program,
                policy,
            )
            .observing_checked(&observer)
            .invoke_root_target(&target, request.arguments(), None, control)
            .map(|(value, _)| value);
            let observation = observer.into_inner().unwrap().map(Observation::Reference);
            (result, observation)
        }
    }
}

fn expected(mirrored: bool) -> NormalizedValue {
    use super::super::value::NormalizedRecord;
    let (left, observed_left, observed_right, last) = if mirrored {
        (211, 4, 211, 7)
    } else {
        (-257, -257, 4, 9)
    };
    NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(vec![
            (Name::new("left").unwrap(), NormalizedValue::I64(left)),
            (
                Name::new("observed").unwrap(),
                NormalizedValue::Record(NormalizedRecord::Structural {
                    fields: Arc::new(vec![
                        (
                            Name::new("left").unwrap(),
                            NormalizedValue::I64(observed_left),
                        ),
                        (
                            Name::new("right").unwrap(),
                            NormalizedValue::I64(observed_right),
                        ),
                    ]),
                }),
            ),
            (
                Name::new("right").unwrap(),
                NormalizedValue::bytes(vec![0, 255, 128, last]),
            ),
        ]),
    })
}

fn healthy(
    fixture: &Fixture,
    engine: Engine,
    executor: &StructuredExecutorHandle,
    mirrored: bool,
    policy: NormalizedRunPolicy,
) -> Observation {
    let storage = Storage::start();
    let (result, observation) = run(
        fixture,
        engine,
        executor,
        Request::healthy(mirrored),
        policy,
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(result.unwrap(), expected(mirrored));
    let observation = observation.expect("healthy invocation records its complete execution");
    observation.clean();
    if let Observation::Reference(o) = &observation {
        assert!(o.canonical_owner_reads > 0);
    }
    storage.transferred();
    storage.clean();
    if let Observation::Vm(o) = &observation {
        assert_eq!(o.parallel_scopes, 2);
        assert_eq!(o.parallel_worker_dispatches, 2);
        assert_eq!(o.parallel_inline_fallbacks, 0);
    }
    assert_eq!(executor.observe().active_dispatches, 0);
    fixture.unchanged();
    observation
}

fn shutdown(executor: &mut StructuredExecutor) {
    let observation = executor.shutdown().unwrap();
    assert!(!observation.dispatch_open);
    assert_eq!(observation.active_dispatches, 0);
    assert_eq!(observation.remaining_workers, 0);
    assert_eq!(observation.joined_workers, observation.workers_started);
}

#[test]
fn associated_parallel_public_imported_selected_outputs_transfer_share_and_quota_retry() {
    let fixture = Fixture::new();
    let mut executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    for engine in [Engine::Vm, Engine::Reference] {
        for mirrored in [false, true] {
            let totals = healthy(
                &fixture,
                engine,
                &handle,
                mirrored,
                NormalizedRunPolicy::foreground(),
            )
            .totals();
            let exact = NormalizedRunPolicy {
                instruction_steps: Some(totals[0]),
                maximum_allocated_bytes: Some(totals[1]),
                maximum_collection_items: Some(totals[2]),
                ..NormalizedRunPolicy::foreground()
            };
            healthy(&fixture, engine, &handle, mirrored, exact);
            for (dimension, total) in totals.iter().enumerate() {
                let mut below = exact;
                let bound = total
                    .checked_sub(1)
                    .expect("healthy execution charges each dimension");
                match dimension {
                    0 => below.instruction_steps = Some(bound),
                    1 => below.maximum_allocated_bytes = Some(bound),
                    2 => below.maximum_collection_items = Some(bound),
                    _ => unreachable!(),
                }
                let storage = Storage::start();
                let (result, observation) = run(
                    &fixture,
                    engine,
                    &handle,
                    Request::healthy(mirrored),
                    below,
                    &ExecutionControl::uncancelled(),
                );
                assert_eq!(
                    result.unwrap_err().class,
                    ExecutionFailureClass::Resource,
                    "{engine:?} dimension {dimension}"
                );
                observation
                    .expect("quota reached the admitted invocation")
                    .clean();
                storage.clean();
                assert_eq!(handle.observe().active_dispatches, 0);
                drop(storage);
                healthy(
                    &fixture,
                    engine,
                    &handle,
                    mirrored,
                    NormalizedRunPolicy::foreground(),
                );
            }
        }
    }
    assert_eq!(handle.observe().workers_started, 1);
    drop(handle);
    shutdown(&mut executor);
}

#[test]
fn associated_parallel_public_imported_traps_join_and_release_each_lane() {
    let fixture = Fixture::new();
    let mut executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    for engine in [Engine::Vm, Engine::Reference] {
        for mirrored in [false, true] {
            for read_phase in [false, true] {
                for left_fails in [false, true] {
                    let storage = Storage::start();
                    let request = Request {
                        mirrored,
                        read_phase,
                        left_fail: left_fails,
                        right_fail: !left_fails,
                        rounds: 0,
                    };
                    let (result, observation) = run(
                        &fixture,
                        engine,
                        &handle,
                        request,
                        NormalizedRunPolicy::foreground(),
                        &ExecutionControl::uncancelled(),
                    );
                    let error = result.unwrap_err();
                    assert_eq!(error.class, ExecutionFailureClass::Trap);
                    assert_eq!(
                        error.code,
                        match engine {
                            Engine::Vm => "normalized_integer_division",
                            Engine::Reference => "reference_integer_division",
                        }
                    );
                    observation
                        .expect("selected mapped body reached its originating trap")
                        .clean();
                    storage.parents_created();
                    storage.clean();
                    assert_eq!(handle.observe().active_dispatches, 0);
                    drop(storage);
                    healthy(
                        &fixture,
                        engine,
                        &handle,
                        mirrored,
                        NormalizedRunPolicy::foreground(),
                    );
                }
            }
        }
    }
    assert_eq!(handle.observe().workers_started, 1);
    drop(handle);
    shutdown(&mut executor);
}

#[test]
fn associated_parallel_public_imported_vm_entered_cancel_joins_and_reuses_executor() {
    let fixture = Fixture::new();
    let mut executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    for mirrored in [false, true] {
        for read_phase in [false, true] {
            let expected_children = if read_phase { 4 } else { 2 };
            // The probe blocks each admitted child until the other has entered. This is
            // a physical entry witness, not a numeric cancellation offset assumption.
            let probe = super::super::vm::ChildProbe::start(fixture.program.value_origin);
            let control = ExecutionControl::uncancelled();
            let storage = Storage::start();
            let parent = std::thread::current().id();
            let (result, observation, entered) = std::thread::scope(|scope| {
                let probe = &probe;
                let control = &control;
                let monitor = scope.spawn(move || {
                    let deadline = Instant::now() + Duration::from_secs(5);
                    loop {
                        let entered = probe.observed();
                        if entered.len() >= expected_children {
                            control.cancel();
                            return entered;
                        }
                        if Instant::now() >= deadline {
                            control.cancel();
                            return entered;
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                });
                let request = Request {
                    read_phase,
                    rounds: 1_000_000,
                    ..Request::healthy(mirrored)
                };
                let (result, observation) = run(
                    &fixture,
                    Engine::Vm,
                    &handle,
                    request,
                    NormalizedRunPolicy::foreground(),
                    control,
                );
                (result, observation, monitor.join().unwrap())
            });
            assert_eq!(
                entered.len(),
                expected_children,
                "both admitted graph children of the selected phase entered before cancellation"
            );
            assert_ne!(
                entered[expected_children - 2].1,
                entered[expected_children - 1].1,
                "each child owns a distinct invocation origin"
            );
            assert!(
                entered[expected_children - 2..]
                    .iter()
                    .any(|(thread, _)| *thread != parent),
                "one child entered on the owned worker"
            );
            let error = result.unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Cancelled);
            assert_eq!(error.code, "execution_cancelled");
            observation
                .expect("entered child cancellation records cleanup")
                .clean();
            storage.clean();
            assert_eq!(handle.observe().active_dispatches, 0);
            drop(storage);
            drop(probe);
            healthy(
                &fixture,
                Engine::Vm,
                &handle,
                mirrored,
                NormalizedRunPolicy::foreground(),
            );
        }
    }
    assert_eq!(handle.observe().workers_started, 1);
    drop(handle);
    shutdown(&mut executor);
}

#[test]
fn associated_parallel_public_imported_reference_entered_cancel_releases_and_retries() {
    let fixture = Fixture::new();
    let mut executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    for mirrored in [false, true] {
        for read_phase in [false, true] {
            let prior_externals = if read_phase { 14 } else { 12 };
            let mut entered_body = false;
            // Parent factories perform 12 externals. In the read phase, the first
            // joined take contributes two terminating burn checks, for a prefix of 14.
            // A greater count proves entry into the selected phase's mapped body.
            // The reference executes children serially; no worker overlap is claimed.
            for cutoff in [0, 16, 64, 256, 1024, 4096, 16384, 65536, 262144] {
                let storage = Storage::start();
                let request = Request {
                    read_phase,
                    rounds: 1_000_000,
                    ..Request::healthy(mirrored)
                };
                let (result, observation) = run(
                    &fixture,
                    Engine::Reference,
                    &handle,
                    request,
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::cancel_after_checks(cutoff),
                );
                let error = result.unwrap_err();
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                assert_eq!(error.code, "execution_cancelled");
                if let Some(observation) = observation {
                    observation.clean();
                    entered_body |= observation.external_calls() > prior_externals;
                } else {
                    // A control failure before state construction records no synthetic observation.
                    assert_eq!(storage.cells.created(), 0);
                    assert_eq!(storage.buffers.created(), 0);
                    assert_eq!(storage.composites.created(), 0);
                }
                storage.clean();
                drop(storage);
                if entered_body {
                    break;
                }
            }
            assert!(
                entered_body,
                "at least one cutoff cancelled inside the actual selected supplier body"
            );
            healthy(
                &fixture,
                Engine::Reference,
                &handle,
                mirrored,
                NormalizedRunPolicy::foreground(),
            );
        }
    }
    assert_eq!(
        handle.observe().workers_started,
        0,
        "canonical reference has no executor dispatch"
    );
    drop(handle);
    shutdown(&mut executor);
}
