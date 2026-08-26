use super::cache::{self, VerificationCache};
use super::model::{
    CacheLookupStatus, CacheObservation, ExecutionKind, Gate, GateReceipt, GateStatus,
    InputSnapshot, MAXIMUM_FAILURE_EXCERPT_BYTES, RuntimeIdentity,
};
use super::registry::GateRegistry;
use super::snapshot;
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof, VerificationDigest};
use crate::process::{self, ProcessSpec, ProcessStatus};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
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
    let order: BTreeMap<String, usize> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect();
    let mut pending: BTreeSet<String> = names.iter().cloned().collect();
    let mut completed: BTreeMap<String, GateReceipt> = BTreeMap::new();
    let (sender, receiver) = mpsc::channel();
    let mut running = 0_usize;

    while !pending.is_empty() || running > 0 {
        let mut progressed = false;
        let candidates: Vec<String> = pending
            .iter()
            .filter(|name| {
                registry.gate(name).is_ok_and(|gate| {
                    gate.dependencies
                        .iter()
                        .all(|dependency| completed.contains_key(dependency))
                })
            })
            .cloned()
            .collect();
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
                        )
                    }))
                    .unwrap_or_else(|_| {
                        Err(DevError::infrastructure("verification worker panicked"))
                    });
                    let _ = sender.send((worker_gate, result));
                });
            match worker {
                Ok(_) => running += 1,
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
            let (gate, result) = receiver.recv().map_err(|error| {
                DevError::infrastructure(format!("verification worker channel closed: {error}"))
            })?;
            running -= 1;
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
) -> Result<GateReceipt, DevError> {
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let fingerprint = gate_fingerprint(repository, gate, snapshot, runtime, dependencies)?;
    let stdout_path = run_directory.join(format!("{}.stdout.log", gate.name));
    let stderr_path = run_directory.join(format!("{}.stderr.log", gate.name));
    if allow_reuse && gate.cacheable {
        let load = verification_cache.load(gate, &fingerprint, &stdout_path, &stderr_path);
        if let Some(cached) = load.cached {
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
                outputs: cached.outputs,
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
        return execute_fresh(
            repository,
            gate,
            fingerprint,
            stdout_path,
            stderr_path,
            started_wall,
            started,
            CacheObservation {
                eligible: true,
                lookup: CacheLookupStatus::Miss,
                reason: Some(load.reason),
                record: None,
                write: None,
            },
        );
    }
    execute_fresh(
        repository,
        gate,
        fingerprint,
        stdout_path,
        stderr_path,
        started_wall,
        started,
        CacheObservation {
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
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_fresh(
    repository: &Path,
    gate: &Gate,
    fingerprint: VerificationDigest,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    started_wall: u128,
    started: Instant,
    cache_observation: CacheObservation,
) -> Result<GateReceipt, DevError> {
    let process = process::run(
        &ProcessSpec {
            command: gate.command.clone(),
            cwd: repository.to_path_buf(),
            environment: process::environment(),
            timeout: gate.timeout,
            maximum_stdout_bytes: gate.maximum_stdout_bytes,
            maximum_stderr_bytes: gate.maximum_stderr_bytes,
            stdout_path: stdout_path.clone(),
            stderr_path: stderr_path.clone(),
            unavailable_exit_code: gate.unavailable_exit_code,
        },
        repository,
    );
    let outputs = cache::output_proofs(repository, &gate.required_outputs)?;
    let mut status = map_status(process.status);
    let mut reason = process.reason.clone();
    if status == GateStatus::Passed && outputs.iter().any(|proof| proof.kind != FileKind::File) {
        status = GateStatus::Failed;
        reason = Some("missing_declared_output".to_owned());
    }
    let evidence_digest =
        cache::gate_evidence_digest(&gate.name, &fingerprint, &process, &outputs)?;
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
        outputs,
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
        outputs: Vec::new(),
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
        outputs: Vec::new(),
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

fn gate_fingerprint(
    repository: &Path,
    gate: &Gate,
    snapshot: &InputSnapshot,
    runtime: &RuntimeIdentity,
    dependencies: &BTreeMap<String, GateReceipt>,
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
        worktree_input_digest: &'a VerificationDigest,
        cargo_lock_digest: &'a VerificationDigest,
        runtime_input_digest: &'a VerificationDigest,
        command_executable: FileProof,
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
    let mut executable = snapshot::executable_proof(repository, &gate.command[0])?;
    if let Some(identity) = gate.identity_command().first() {
        executable.path.clone_from(identity);
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
