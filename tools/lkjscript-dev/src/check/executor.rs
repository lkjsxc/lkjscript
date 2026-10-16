use super::cache::{self, VerificationCache};
use super::cargo_origin;
use super::model::{
    CacheLookupStatus, CacheObservation, ExecutableProof, ExecutionKind, Gate, GateReceipt,
    GateStatus, InputSnapshot, MAXIMUM_FAILURE_EXCERPT_BYTES, RuntimeIdentity,
};
use super::registry::GateRegistry;
use super::snapshot;
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof, VerificationDigest};
use crate::process::{self, ProcessSpec, ProcessStatus};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub(crate) struct ExecutionOptions<'a> {
    pub(crate) repository: &'a Path,
    pub(crate) run_directory: &'a Path,
    pub(crate) snapshot: &'a InputSnapshot,
    pub(crate) runtime: &'a RuntimeIdentity,
    pub(crate) maximum_workers: usize,
    pub(crate) allow_reuse: bool,
    pub(crate) fresh_reason: &'a str,
    pub(crate) cache: &'a VerificationCache,
    pub(super) origin: Option<Arc<cargo_origin::Run>>,
    pub(super) live: Option<Arc<super::compact::Run>>,
}

// An early scheduler error must not detach still-running gate owners. Normal completion removes
// and fallibly joins each worker before its receipt can count; this fallback joins only error exits.
#[derive(Default)]
struct RemainingWorkers(BTreeMap<String, thread::JoinHandle<()>>);

impl Drop for RemainingWorkers {
    fn drop(&mut self) {
        for (_, worker) in std::mem::take(&mut self.0) {
            let _ = worker.join();
        }
    }
}

// Readers of live target outputs must not acquire their long-lived read lease
// ahead of a pending Cargo writer. Keep prerequisite readers eligible so a
// selected writer can always reach its own declared inputs without deadlock.
struct CargoSchedule {
    writers: BTreeSet<String>,
    prerequisites: BTreeSet<String>,
}

impl CargoSchedule {
    fn new(registry: &GateRegistry, names: &[String]) -> Result<Self, DevError> {
        let writers: Vec<_> = names
            .iter()
            .filter(|name| registry.gate(name).is_ok_and(cargo_origin::is_writer))
            .cloned()
            .collect();
        Ok(Self {
            prerequisites: registry.closure(&writers)?.into_iter().collect(),
            writers: writers.into_iter().collect(),
        })
    }

    fn allows(&self, name: &str, complete: impl Fn(&str) -> bool) -> bool {
        self.prerequisites.contains(name) || self.writers.iter().all(|writer| complete(writer))
    }
}

pub(crate) fn execute_dag(
    registry: &GateRegistry,
    names: &[String],
    options: &ExecutionOptions<'_>,
) -> Result<Vec<GateReceipt>, DevError> {
    if options.maximum_workers == 0 || options.maximum_workers > super::model::MAXIMUM_WORKERS {
        return Err(DevError::usage(format!(
            "worker count must be between 1 and {}",
            super::model::MAXIMUM_WORKERS
        )));
    }
    let origin = match &options.origin {
        Some(origin) => Some(origin.clone()),
        None if options.live.is_none()
            && names
                .iter()
                .any(|name| registry.gate(name).is_ok_and(cargo_origin::is_writer)) =>
        {
            let commands = registry.input_commands(names)?;
            Some(Arc::new(cargo_origin::Run::with_commands_for_run(
                options.repository,
                &super::cargo_origin_context::Context::target(options.repository)?,
                options.snapshot,
                options.runtime,
                commands.into_iter().collect(),
                options.run_directory,
            )?))
        }
        None => None,
    };
    let order: BTreeMap<String, usize> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect();
    let cargo_schedule = options
        .live
        .as_ref()
        .map(|_| CargoSchedule::new(registry, names))
        .transpose()?;
    let mut pending: BTreeSet<String> = names.iter().cloned().collect();
    let mut completed: BTreeMap<String, GateReceipt> = BTreeMap::new();
    let (sender, receiver) = mpsc::channel();
    let mut running = 0_usize;
    let mut workers = RemainingWorkers::default();

    while !pending.is_empty() || running > 0 {
        let mut progressed = false;
        let mut candidates: Vec<String> = pending
            .iter()
            .filter(|name| {
                cargo_schedule.as_ref().is_none_or(|schedule| {
                    schedule.allows(name, |writer| completed.contains_key(writer))
                })
            })
            .filter(|name| {
                registry.gate(name).is_ok_and(|gate| {
                    gate.dependencies
                        .iter()
                        .all(|dependency| completed.contains_key(dependency))
                })
            })
            .cloned()
            .collect();
        // Ready long-running owners start first; dependencies and complete coverage
        // are unchanged. Alphabetical order otherwise delays service lifecycles.
        candidates.sort_by_key(|name| match name.as_str() {
            "workspace_tests" | "offline_packages" => 0,
            "release_command_lifecycle" => 1,
            "service_acceptance" | "stateful_http_application" => 2,
            "pure_tail" => 3,
            _ => 4,
        });
        for name in candidates {
            if running >= options.maximum_workers {
                break;
            }
            let gate = registry.gate(&name)?.clone();
            let dependencies: BTreeMap<String, GateReceipt> = gate
                .dependencies
                .iter()
                .filter_map(|dependency| {
                    completed
                        .get(dependency)
                        .cloned()
                        .map(|receipt| (dependency.clone(), receipt))
                })
                .collect();
            pending.remove(&name);
            progressed = true;
            if dependencies.values().any(|receipt| !receipt.passed()) {
                let skipped = skipped_receipt(
                    options.repository,
                    &gate,
                    options.snapshot,
                    options.runtime,
                    &dependencies,
                )?;
                completed.insert(name, skipped);
                continue;
            }

            let sender = sender.clone();
            let repository = options.repository.to_path_buf();
            let run_directory = options.run_directory.to_path_buf();
            let snapshot = options.snapshot.clone();
            let runtime = options.runtime.clone();
            let cache = options.cache.clone();
            let allow_reuse = options.allow_reuse;
            let fresh_reason = options.fresh_reason.to_owned();
            let worker_gate = gate.clone();
            let origin = origin.clone();
            let live = options.live.clone();
            let worker = thread::Builder::new()
                .name(format!("check-{}", gate.name))
                .spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        execute_gate(
                            &repository,
                            &run_directory,
                            &worker_gate,
                            &snapshot,
                            &runtime,
                            &dependencies,
                            allow_reuse,
                            &fresh_reason,
                            &cache,
                            origin.as_deref(),
                            live.as_deref(),
                        )
                    }))
                    .unwrap_or_else(|_| {
                        Err(DevError::infrastructure("verification worker panicked"))
                    });
                    let _ = sender.send((worker_gate, result));
                });
            match worker {
                Ok(worker) => {
                    workers.0.insert(gate.name.clone(), worker);
                    running += 1;
                }
                Err(error) => {
                    let receipt = infrastructure_receipt(
                        &gate,
                        DevError::infrastructure(format!(
                            "spawn verification worker '{}': {error}",
                            gate.name
                        )),
                    );
                    completed.insert(name, receipt);
                }
            }
        }

        if running > 0 {
            let (gate, mut result) = receiver.recv().map_err(|error| {
                DevError::infrastructure(format!("verification worker channel closed: {error}"))
            })?;
            running -= 1;
            let worker = workers
                .0
                .remove(&gate.name)
                .ok_or_else(|| DevError::infrastructure("verification worker handle is absent"))?;
            if worker.join().is_err() {
                result = Err(DevError::infrastructure(
                    "verification worker failed during joined cleanup",
                ));
            }
            let receipt = result.unwrap_or_else(|error| infrastructure_receipt(&gate, error));
            completed.insert(gate.name.clone(), receipt);
            progressed = true;
        }
        if !progressed && !pending.is_empty() {
            return Err(DevError::infrastructure(
                "verification DAG scheduler made no progress",
            ));
        }
    }

    let mut results = Vec::with_capacity(names.len());
    for name in names {
        let receipt = completed.remove(name).ok_or_else(|| {
            DevError::infrastructure(format!("gate '{name}' produced no receipt"))
        })?;
        results.push(receipt);
    }
    results.sort_by_key(|receipt| order.get(&receipt.name).copied().unwrap_or(usize::MAX));
    Ok(results)
}

#[cfg(test)]
mod scheduling_tests {
    use super::*;

    #[test]
    fn live_output_readers_wait_for_all_selected_writers_but_not_their_prerequisites() {
        let gate = |name: &str, command: &[&str], dependencies: &[&str]| {
            let mut gate = Gate::new(name, command.iter().map(|value| (*value).into()).collect());
            gate.dependencies = dependencies.iter().map(|value| (*value).into()).collect();
            gate
        };
        let registry = GateRegistry::new(vec![
            gate("fmt", &["cargo", "fmt"], &[]),
            gate("source", &["source-export"], &["fmt"]),
            gate("build", &["cargo", "build"], &["source"]),
            gate("lint", &["cargo", "clippy"], &["fmt"]),
            gate("slow", &["semantic-fixtures"], &["fmt"]),
        ])
        .unwrap();
        let names = registry
            .closure(&["build".into(), "lint".into(), "slow".into()])
            .unwrap();
        let schedule = CargoSchedule::new(&registry, &names).unwrap();
        for completed in [vec![], vec!["fmt"], vec!["fmt", "source", "build"]] {
            let complete = |name: &str| completed.contains(&name);
            assert!(schedule.allows("fmt", complete));
            assert!(schedule.allows("source", complete));
            assert!(schedule.allows("build", complete));
            assert!(schedule.allows("lint", complete));
            assert!(!schedule.allows("slow", complete));
        }
        assert!(schedule.allows("slow", |name| ["build", "lint"].contains(&name)));
        // An unselected Cargo gate cannot delay a standalone reader profile.
        let standalone = registry.closure(&["slow".into()]).unwrap();
        let schedule = CargoSchedule::new(&registry, &standalone).unwrap();
        assert!(schedule.allows("slow", |_| false));
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_gate(
    repository: &Path,
    run_directory: &Path,
    gate: &Gate,
    snapshot: &InputSnapshot,
    runtime: &RuntimeIdentity,
    dependencies: &BTreeMap<String, GateReceipt>,
    allow_reuse: bool,
    fresh_reason: &str,
    verification_cache: &VerificationCache,
    origin: Option<&cargo_origin::Run>,
    live: Option<&super::compact::Run>,
) -> Result<GateReceipt, DevError> {
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let _access = live.map(|run| run.enter(gate)).transpose()?;
    let mut transaction = if live.is_none() && cargo_origin::is_writer(gate) {
        Some(
            origin
                .ok_or_else(|| DevError::infrastructure("Cargo target owner is absent"))?
                .begin(gate)?,
        )
    } else {
        None
    };
    let inputs = expected_input_proofs(repository, gate, dependencies)?;
    verify_live_inputs(repository, gate, &inputs)?;
    let command = selected_command(repository, gate, &inputs)?;
    let selected = snapshot::observe_executable(repository, &command)?;
    let fingerprint = fingerprint_with_executable(
        repository,
        gate,
        snapshot,
        runtime,
        dependencies,
        selected.proof,
    )?;
    let stdout_path = run_directory.join(format!("{}.stdout.log", gate.name));
    let stderr_path = run_directory.join(format!("{}.stderr.log", gate.name));
    if allow_reuse && gate.cacheable && live.is_none() {
        let load = verification_cache.load(gate, &fingerprint, &stdout_path, &stderr_path);
        if let Some(cached) = load.cached {
            if let Some(transaction) = &mut transaction {
                transaction.finish_unchanged()?;
            }
            let retained_outputs =
                retain_outputs(repository, run_directory, gate, &cached.outputs)?;
            return Ok(GateReceipt {
                name: gate.name.clone(),
                status: GateStatus::Passed,
                execution: ExecutionKind::Reused,
                command: gate.command.clone(),
                dependencies: gate.dependencies.clone(),
                failed_dependencies: Vec::new(),
                started_unix_nanoseconds: started_wall,
                completed_unix_nanoseconds: unix_nanoseconds()?,
                elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
                process: Some(cached.process),
                inputs,
                outputs: cached.outputs,
                retained_outputs,
                cargo_origin: cached.cargo_origin,
                input_fingerprint: fingerprint,
                evidence_digest: cached.evidence_digest,
                cache: CacheObservation {
                    eligible: true,
                    lookup: CacheLookupStatus::Hit,
                    reason: Some(format!(
                        "source_elapsed_nanoseconds={}",
                        cached.source_elapsed_nanoseconds
                    )),
                    record: Some(cached.record),
                    write: None,
                },
                reason: None,
                stdout_excerpt: None,
                stderr_excerpt: None,
            });
        }
        return execute_fresh_inner(
            FreshInput {
                repository,
                gate,
                inputs,
                program: selected.path.as_deref(),
                fingerprint,
                stdout_path,
                stderr_path,
                started_wall,
                started,
                cache_observation: CacheObservation {
                    eligible: true,
                    lookup: CacheLookupStatus::Miss,
                    reason: Some(load.reason),
                    record: None,
                    write: None,
                },
            },
            transaction,
            live,
        );
    }
    execute_fresh_inner(
        FreshInput {
            repository,
            gate,
            inputs,
            program: selected.path.as_deref(),
            fingerprint,
            stdout_path,
            stderr_path,
            started_wall,
            started,
            cache_observation: CacheObservation {
                eligible: gate.cacheable,
                lookup: CacheLookupStatus::Bypassed,
                reason: Some(if gate.cacheable {
                    fresh_reason.to_owned()
                } else {
                    "gate_not_cacheable".to_owned()
                }),
                record: None,
                write: None,
            },
        },
        transaction,
        live,
    )
}

pub(super) struct FreshInput<'a> {
    pub(super) repository: &'a Path,
    pub(super) gate: &'a Gate,
    pub(super) inputs: Vec<FileProof>,
    pub(super) program: Option<&'a Path>,
    pub(super) fingerprint: VerificationDigest,
    pub(super) stdout_path: PathBuf,
    pub(super) stderr_path: PathBuf,
    pub(super) started_wall: u128,
    pub(super) started: Instant,
    pub(super) cache_observation: CacheObservation,
}

pub(super) fn expected_input_proofs(
    repository: &Path,
    gate: &Gate,
    dependencies: &BTreeMap<String, GateReceipt>,
) -> Result<Vec<FileProof>, DevError> {
    let mut inputs = gate
        .required_inputs
        .iter()
        .map(|input| {
            let producer = dependencies.get(&input.producer).ok_or_else(|| {
                DevError::corrupt("declared input has no admitted dependency producer")
            })?;
            let path = evidence::relative(repository, &input.path);
            let matches: Vec<_> = producer
                .outputs
                .iter()
                .filter(|proof| proof.path == path)
                .collect();
            if !producer.passed() || matches.len() != 1 {
                return Err(DevError::corrupt(
                    "declared input producer output is absent or ambiguous",
                ));
            }
            let proof = matches[0];
            if proof.kind != FileKind::File || proof.digest.is_none() || proof.link_target.is_some()
            {
                return Err(DevError::corrupt(
                    "declared input is not a regular admitted artifact",
                ));
            }
            Ok(proof.clone())
        })
        .collect::<Result<Vec<_>, DevError>>()?;
    if let Some(test) = &gate.cargo_test {
        inputs.extend(super::cargo_tests::input_proofs(
            repository,
            test,
            dependencies,
        )?);
    }
    Ok(inputs)
}

fn selected_command(
    repository: &Path,
    gate: &Gate,
    inputs: &[FileProof],
) -> Result<String, DevError> {
    if gate.cargo_test.is_some() && !super::workspace_tests::selected(gate) {
        let proof = inputs
            .last()
            .ok_or_else(|| DevError::corrupt("selected test input is absent"))?;
        Ok(repository.join(&proof.path).to_string_lossy().into_owned())
    } else {
        Ok(gate.command[0].clone())
    }
}

fn verify_live_inputs(
    repository: &Path,
    gate: &Gate,
    inputs: &[FileProof],
) -> Result<(), DevError> {
    let test_inputs = if super::workspace_tests::selected(gate) {
        super::workspace_tests::TARGET_COUNT
    } else {
        usize::from(gate.cargo_test.is_some())
    };
    if inputs.len() != gate.required_inputs.len() + test_inputs {
        return Err(DevError::corrupt("declared input evidence is incomplete"));
    }
    if gate
        .required_inputs
        .iter()
        .zip(inputs)
        .any(|(input, proof)| proof.path != evidence::relative(repository, &input.path))
    {
        return Err(DevError::corrupt("declared input path was substituted"));
    }
    for expected in inputs {
        let path = repository.join(&expected.path);
        let observed = evidence::proof(&path, expected.path.clone())?;
        if observed != *expected {
            return Err(DevError::corrupt(
                "declared input differs from its admitted producer output",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn execute_fresh_with_origin(
    input: FreshInput<'_>,
    origin: &cargo_origin::Run,
) -> Result<GateReceipt, DevError> {
    let transaction = if cargo_origin::is_writer(input.gate) {
        Some(origin.begin(input.gate)?)
    } else {
        None
    };
    execute_fresh_inner(input, transaction, None)
}

fn execute_fresh_inner(
    input: FreshInput<'_>,
    mut transaction: Option<cargo_origin::Transaction<'_>>,
    live: Option<&super::compact::Run>,
) -> Result<GateReceipt, DevError> {
    let FreshInput {
        repository,
        gate,
        inputs,
        program,
        fingerprint,
        stdout_path,
        stderr_path,
        started_wall,
        started,
        cache_observation,
    } = input;
    verify_live_inputs(repository, gate, &inputs)?;
    let mut environment = process::environment();
    environment.extend(gate.environment.clone());
    let mut command = gate.command.clone();
    if super::workspace_tests::selected(gate) {
        command.push(super::workspace_tests::request(
            repository,
            &stdout_path,
            &inputs,
        )?);
    }
    let process = process::run_selected(
        &ProcessSpec {
            command,
            cwd: repository.to_path_buf(),
            environment,
            timeout: gate.timeout,
            maximum_stdout_bytes: gate.maximum_stdout_bytes,
            maximum_stderr_bytes: gate.maximum_stderr_bytes,
            stdout_path: stdout_path.clone(),
            stderr_path: stderr_path.clone(),
            unavailable_exit_code: gate.unavailable_exit_code,
        },
        repository,
        program,
    );
    let outputs = cache::output_proofs(repository, &gate.required_outputs)?;
    let run_directory = stdout_path
        .parent()
        .ok_or_else(|| DevError::infrastructure("gate stdout has no parent"))?;
    let retained_outputs = if live.is_none() {
        retain_outputs(repository, run_directory, gate, &outputs)?
    } else {
        Vec::new()
    };
    let mut status = map_status(process.status);
    let mut reason = process.reason.clone();
    if status == GateStatus::Passed
        && let Err(error) = verify_live_inputs(repository, gate, &inputs)
    {
        status = GateStatus::Failed;
        reason = Some(format!("declared_input_binding:{}", error.message()));
    }
    if status == GateStatus::Passed && outputs.iter().any(|proof| proof.kind != FileKind::File) {
        status = GateStatus::Failed;
        reason = Some("missing_declared_output".to_owned());
    }
    if status == GateStatus::Passed
        && let Err(error) =
            super::cargo_outputs::verify(repository, gate, &stdout_path, &process.stdout)
    {
        status = GateStatus::Failed;
        reason = Some(format!("cargo_output_binding:{}", error.message()));
    }
    if status == GateStatus::Passed
        && let Err(error) =
            super::cargo_tests::verify_result(repository, gate, &stdout_path, &inputs)
    {
        status = GateStatus::Failed;
        reason = Some(format!("cargo_test_result:{}", error.message()));
    }
    let mut cargo_origin = None;
    if status == GateStatus::Passed
        && let Some(live) = live
    {
        match live.finish(gate, &process, run_directory) {
            Ok(proof) => cargo_origin = proof,
            Err(error) => {
                status = GateStatus::Failed;
                reason = Some(format!("cargo_output_binding:{}", error.message()));
            }
        }
    }
    if status == GateStatus::Passed
        && let Some(transaction) = &mut transaction
    {
        match transaction.finish(gate, &process, run_directory) {
            Ok(proof) => cargo_origin = proof,
            Err(error) => {
                status = GateStatus::Failed;
                reason = Some(format!("cargo_output_binding:{}", error.message()));
            }
        }
    }
    let evidence_digest = cache::gate_evidence_digest(
        &gate.name,
        &fingerprint,
        &process,
        &inputs,
        &outputs,
        cargo_origin.as_ref(),
    )?;
    let (stdout_excerpt, stderr_excerpt) = if status == GateStatus::Passed {
        (None, None)
    } else {
        (
            process::excerpt(&stdout_path, MAXIMUM_FAILURE_EXCERPT_BYTES).ok(),
            process::excerpt(&stderr_path, MAXIMUM_FAILURE_EXCERPT_BYTES).ok(),
        )
    };
    Ok(GateReceipt {
        name: gate.name.clone(),
        status,
        execution: ExecutionKind::Fresh,
        command: gate.command.clone(),
        dependencies: gate.dependencies.clone(),
        failed_dependencies: Vec::new(),
        started_unix_nanoseconds: started_wall,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        process: Some(process),
        inputs,
        outputs,
        retained_outputs,
        cargo_origin,
        input_fingerprint: fingerprint,
        evidence_digest,
        cache: cache_observation,
        reason,
        stdout_excerpt,
        stderr_excerpt,
    })
}

fn skipped_receipt(
    repository: &Path,
    gate: &Gate,
    snapshot: &InputSnapshot,
    runtime: &RuntimeIdentity,
    dependencies: &BTreeMap<String, GateReceipt>,
) -> Result<GateReceipt, DevError> {
    let failed_dependencies: Vec<String> = dependencies
        .iter()
        .filter(|(_, receipt)| !receipt.passed())
        .map(|(name, _)| name.clone())
        .collect();
    let fingerprint = gate_fingerprint(repository, gate, snapshot, runtime, dependencies)?;
    #[derive(Serialize)]
    struct SkippedIdentity<'a> {
        gate: &'a str,
        fingerprint: &'a VerificationDigest,
        failed_dependencies: &'a [String],
    }
    let evidence_bytes = serde_json::to_vec(&SkippedIdentity {
        gate: &gate.name,
        fingerprint: &fingerprint,
        failed_dependencies: &failed_dependencies,
    })
    .map_err(|error| DevError::infrastructure(format!("encode skipped evidence: {error}")))?;
    let now = unix_nanoseconds()?;
    Ok(GateReceipt {
        name: gate.name.clone(),
        status: GateStatus::Skipped,
        execution: ExecutionKind::Skipped,
        command: gate.command.clone(),
        dependencies: gate.dependencies.clone(),
        failed_dependencies,
        started_unix_nanoseconds: now,
        completed_unix_nanoseconds: now,
        elapsed_nanoseconds: 0,
        process: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        retained_outputs: Vec::new(),
        cargo_origin: None,
        input_fingerprint: fingerprint,
        evidence_digest: VerificationDigest::of(&evidence_bytes),
        cache: CacheObservation {
            eligible: false,
            lookup: CacheLookupStatus::NotAttempted,
            reason: Some("failed_prerequisite".to_owned()),
            record: None,
            write: None,
        },
        reason: Some("failed_prerequisite".to_owned()),
        stdout_excerpt: None,
        stderr_excerpt: None,
    })
}

fn infrastructure_receipt(gate: &Gate, error: DevError) -> GateReceipt {
    let now = unix_nanoseconds().unwrap_or(0);
    let fingerprint = VerificationDigest::of(error.to_string().as_bytes());
    GateReceipt {
        name: gate.name.clone(),
        status: GateStatus::InfrastructureFailure,
        execution: ExecutionKind::Fresh,
        command: gate.command.clone(),
        dependencies: gate.dependencies.clone(),
        failed_dependencies: Vec::new(),
        started_unix_nanoseconds: now,
        completed_unix_nanoseconds: now,
        elapsed_nanoseconds: 0,
        process: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        retained_outputs: Vec::new(),
        cargo_origin: None,
        input_fingerprint: fingerprint.clone(),
        evidence_digest: VerificationDigest::of(
            format!("{}:{fingerprint}", error.kind()).as_bytes(),
        ),
        cache: CacheObservation {
            eligible: false,
            lookup: CacheLookupStatus::NotAttempted,
            reason: Some("checker_infrastructure".to_owned()),
            record: None,
            write: None,
        },
        reason: Some(format!("{}:{}", error.kind(), error.message())),
        stdout_excerpt: None,
        stderr_excerpt: None,
    }
}

fn retain_outputs(
    repository: &Path,
    run_directory: &Path,
    gate: &Gate,
    outputs: &[FileProof],
) -> Result<Vec<FileProof>, DevError> {
    let mut retained = Vec::with_capacity(outputs.len());
    for (index, original) in outputs.iter().enumerate() {
        if original.kind != FileKind::File {
            continue;
        }
        let source = repository.join(&original.path);
        let directory = run_directory.join("retained").join(&gate.name);
        std::fs::create_dir_all(&directory)?;
        let path = directory.join(index.to_string());
        let mut input = std::fs::File::open(&source)?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        std::io::copy(&mut input, &mut output)?;
        output.set_permissions(input.metadata()?.permissions())?;
        output.sync_all()?;
        let proof = evidence::proof(&path, evidence::relative(repository, &path))?;
        let mut comparison = proof.clone();
        comparison.path.clone_from(&original.path);
        if comparison != *original {
            return Err(DevError::infrastructure(
                "gate output changed while preserving its original evidence",
            ));
        }
        retained.push(proof);
    }
    Ok(retained)
}

pub(super) fn gate_fingerprint(
    repository: &Path,
    gate: &Gate,
    snapshot: &InputSnapshot,
    runtime: &RuntimeIdentity,
    dependencies: &BTreeMap<String, GateReceipt>,
) -> Result<VerificationDigest, DevError> {
    let command = if gate.cargo_test.is_some() && dependencies.values().all(GateReceipt::passed) {
        selected_command(
            repository,
            gate,
            &expected_input_proofs(repository, gate, dependencies)?,
        )?
    } else {
        gate.command[0].clone()
    };
    fingerprint_with_executable(
        repository,
        gate,
        snapshot,
        runtime,
        dependencies,
        snapshot::executable_proof(repository, &command)?,
    )
}

fn fingerprint_with_executable(
    repository: &Path,
    gate: &Gate,
    snapshot: &InputSnapshot,
    runtime: &RuntimeIdentity,
    dependencies: &BTreeMap<String, GateReceipt>,
    mut executable: ExecutableProof,
) -> Result<VerificationDigest, DevError> {
    #[derive(Serialize)]
    struct DependencyIdentity<'a> {
        status: GateStatus,
        input_fingerprint: &'a VerificationDigest,
        evidence_digest: &'a VerificationDigest,
        outputs: &'a [FileProof],
    }
    #[derive(Serialize)]
    struct FingerprintIdentity<'a> {
        cache_contract_version: u32,
        gate: &'a str,
        command: &'a [String],
        cwd: &'static str,
        dependencies: BTreeMap<&'a str, DependencyIdentity<'a>>,
        timeout_nanoseconds: u128,
        maximum_stdout_bytes: u64,
        maximum_stderr_bytes: u64,
        required_outputs: Vec<String>,
        required_inputs: &'a [super::model::GateInput],
        cargo_test: &'a Option<super::model::CargoTestInput>,
        environment: &'a BTreeMap<String, String>,
        worktree_input_digest: &'a VerificationDigest,
        cargo_lock_digest: &'a VerificationDigest,
        runtime_input_digest: &'a VerificationDigest,
        command_executable: ExecutableProof,
    }
    let dependency_identity = dependencies
        .iter()
        .map(|(name, receipt)| {
            (
                name.as_str(),
                DependencyIdentity {
                    status: receipt.status,
                    input_fingerprint: &receipt.input_fingerprint,
                    evidence_digest: &receipt.evidence_digest,
                    outputs: &receipt.outputs,
                },
            )
        })
        .collect();
    if let Some(identity) = gate.identity_command().first() {
        executable.relabel(identity);
    }
    let identity = FingerprintIdentity {
        cache_contract_version: super::model::CACHE_CONTRACT_VERSION,
        gate: &gate.name,
        command: gate.identity_command(),
        cwd: ".",
        dependencies: dependency_identity,
        timeout_nanoseconds: gate.timeout.as_nanos(),
        maximum_stdout_bytes: gate.maximum_stdout_bytes,
        maximum_stderr_bytes: gate.maximum_stderr_bytes,
        required_inputs: &gate.required_inputs,
        cargo_test: &gate.cargo_test,
        environment: &gate.environment,
        required_outputs: gate
            .required_outputs
            .iter()
            .map(|path| evidence::relative(repository, path))
            .collect(),
        worktree_input_digest: &snapshot.digest,
        cargo_lock_digest: &snapshot.cargo_lock_digest,
        runtime_input_digest: &runtime.digest,
        command_executable: executable,
    };
    let bytes = serde_json::to_vec(&identity)
        .map_err(|error| DevError::infrastructure(format!("encode gate fingerprint: {error}")))?;
    Ok(VerificationDigest::of(&bytes))
}

fn map_status(status: ProcessStatus) -> GateStatus {
    match status {
        ProcessStatus::Passed => GateStatus::Passed,
        ProcessStatus::Failed => GateStatus::Failed,
        ProcessStatus::Unavailable => GateStatus::Unavailable,
        ProcessStatus::Timeout => GateStatus::Timeout,
        ProcessStatus::OutputExhausted => GateStatus::OutputExhausted,
        ProcessStatus::Signaled => GateStatus::Signaled,
        ProcessStatus::InfrastructureFailure => GateStatus::InfrastructureFailure,
    }
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock before epoch: {error}")))
}

fn duration_nanoseconds(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod worker_cleanup_tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn early_scheduler_error_joins_remaining_worker_before_return() {
        let completed = Arc::new(AtomicBool::new(false));
        let observation = completed.clone();
        let result: Result<(), DevError> = {
            let mut workers = RemainingWorkers::default();
            workers.0.insert(
                "owned".to_owned(),
                thread::spawn(move || {
                    thread::sleep(std::time::Duration::from_millis(20));
                    observation.store(true, Ordering::SeqCst);
                }),
            );
            Err(DevError::infrastructure("injected scheduler failure"))
        };
        assert!(result.is_err());
        assert!(
            completed.load(Ordering::SeqCst),
            "worker escaped failed scheduler"
        );
    }
}

#[cfg(test)]
#[path = "declared_inputs_tests.rs"]
mod declared_inputs_tests;

#[cfg(test)]
#[path = "cargo_outputs/production_gate_tests.rs"]
mod cargo_output_tests;

#[cfg(test)]
#[path = "cargo_outputs/cache_tests.rs"]
mod cargo_cache_tests;
