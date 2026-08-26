use super::cache::VerificationCache;
use super::executor::{self, ExecutionOptions};
use super::model::{
    CHECK_CONTRACT_VERSION, ExecutionKind, Gate, GateStatus, InputSnapshot, PlatformIdentity,
    RuntimeIdentity,
};
use super::registry::GateRegistry;
use crate::error::DevError;
use crate::evidence::{self, AtomicCheckpoint, VerificationDigest};
use crate::process::{self, ProcessSpec, ProcessStatus};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct SelfTestReceipt {
    contract_version: u32,
    kind: &'static str,
    status: &'static str,
    elapsed_nanoseconds: u64,
    process_statuses: BTreeMap<String, ProcessStatus>,
    process_reasons: BTreeMap<String, Option<String>>,
    dag_statuses: BTreeMap<String, GateStatus>,
    graph_rejections: BTreeMap<String, bool>,
    shared_prerequisite_executions: usize,
    cache_initial_miss: String,
    cache_hit: String,
    cache_corrupt: String,
    atomic_replacement_fail_closed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<String>,
}

pub(crate) fn run(machine: bool) -> Result<u8, DevError> {
    let repository = super::repository_root()?;
    let run = super::new_run_directory(&repository, "self-test-")?;
    let directory = run.path();
    let started = Instant::now();
    let receipt = match run_inner(&repository, directory, started) {
        Ok(receipt) => receipt,
        Err(error) => SelfTestReceipt {
            contract_version: CHECK_CONTRACT_VERSION,
            kind: "checker_self_test",
            status: "failed",
            elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
            process_statuses: BTreeMap::new(),
            process_reasons: BTreeMap::new(),
            dag_statuses: BTreeMap::new(),
            graph_rejections: BTreeMap::new(),
            shared_prerequisite_executions: 0,
            cache_initial_miss: "not_run".to_owned(),
            cache_hit: "not_run".to_owned(),
            cache_corrupt: "not_run".to_owned(),
            atomic_replacement_fail_closed: false,
            failure: Some(format!("{}:{}", error.kind(), error.message())),
        },
    };
    let path = directory.join("receipt.json");
    let published = evidence::publish_json(&path, &receipt)?;
    let relative = evidence::relative(&repository, &published.path);
    if machine {
        #[derive(Serialize)]
        struct Summary<'a> {
            contract_version: u32,
            status: &'a str,
            receipt: &'a str,
            receipt_bytes: u64,
            receipt_digest: &'a VerificationDigest,
        }
        println!(
            "{}",
            serde_json::to_string(&Summary {
                contract_version: CHECK_CONTRACT_VERSION,
                status: receipt.status,
                receipt: &relative,
                receipt_bytes: published.bytes,
                receipt_digest: &published.digest,
            })
            .map_err(|error| DevError::infrastructure(format!(
                "encode self-test summary: {error}"
            )))?
        );
    } else {
        println!(
            "check self-test {}; receipt={} digest={}",
            receipt.status, relative, published.digest
        );
    }
    Ok(if receipt.status == "passed" { 0 } else { 1 })
}

fn run_inner(
    repository: &Path,
    directory: &Path,
    started: Instant,
) -> Result<SelfTestReceipt, DevError> {
    super::rotate_runs(repository, directory, "self-test-")?;
    let executable = std::env::current_exe().map_err(|error| {
        DevError::infrastructure(format!("resolve self-test executable: {error}"))
    })?;
    let executable_text = executable.to_string_lossy().into_owned();
    let cases = [
        ("pass", "pass", Duration::from_secs(2), 1_024, 1_024),
        ("failure", "fail", Duration::from_secs(2), 1_024, 1_024),
        ("timeout", "sleep", Duration::from_millis(50), 1_024, 1_024),
        ("stdout", "stdout", Duration::from_secs(2), 1_024, 8_192),
        ("stderr", "stderr", Duration::from_secs(2), 8_192, 1_024),
    ];
    let mut observations = BTreeMap::new();
    for (name, fixture, timeout, stdout_limit, stderr_limit) in cases {
        observations.insert(
            name.to_owned(),
            process::run(
                &ProcessSpec {
                    command: vec![
                        executable_text.clone(),
                        "__fixture".to_owned(),
                        fixture.to_owned(),
                    ],
                    cwd: repository.to_path_buf(),
                    environment: BTreeMap::new(),
                    timeout,
                    maximum_stdout_bytes: stdout_limit,
                    maximum_stderr_bytes: stderr_limit,
                    stdout_path: directory.join(format!("{name}.stdout.log")),
                    stderr_path: directory.join(format!("{name}.stderr.log")),
                    unavailable_exit_code: None,
                },
                repository,
            ),
        );
    }
    observations.insert(
        "unavailable".to_owned(),
        process::run(
            &ProcessSpec {
                command: vec!["lkjscript-intentionally-absent-command".to_owned()],
                cwd: repository.to_path_buf(),
                environment: BTreeMap::new(),
                timeout: Duration::from_secs(2),
                maximum_stdout_bytes: 1_024,
                maximum_stderr_bytes: 1_024,
                stdout_path: directory.join("unavailable.stdout.log"),
                stderr_path: directory.join("unavailable.stderr.log"),
                unavailable_exit_code: None,
            },
            repository,
        ),
    );
    let process_statuses = observations
        .iter()
        .map(|(name, observation)| (name.clone(), observation.status))
        .collect::<BTreeMap<_, _>>();
    let process_reasons = observations
        .iter()
        .map(|(name, observation)| (name.clone(), observation.reason.clone()))
        .collect::<BTreeMap<_, _>>();
    let process_passed = process_statuses
        == BTreeMap::from([
            ("failure".to_owned(), ProcessStatus::Failed),
            ("pass".to_owned(), ProcessStatus::Passed),
            ("stderr".to_owned(), ProcessStatus::OutputExhausted),
            ("stdout".to_owned(), ProcessStatus::OutputExhausted),
            ("timeout".to_owned(), ProcessStatus::Timeout),
            ("unavailable".to_owned(), ProcessStatus::Unavailable),
        ])
        && observations
            .get("stdout")
            .is_some_and(|value| value.stdout_limit_exhausted && !value.stderr_limit_exhausted)
        && observations
            .get("stderr")
            .is_some_and(|value| !value.stdout_limit_exhausted && value.stderr_limit_exhausted);

    let graph_rejections = graph_rejections();
    let (dag_statuses, shared_executions) =
        dag_test(repository, directory, &executable, &executable_text)?;
    let dag_passed = dag_statuses
        == BTreeMap::from([
            ("blocked".to_owned(), GateStatus::Skipped),
            ("child_one".to_owned(), GateStatus::Passed),
            ("child_two".to_owned(), GateStatus::Passed),
            ("dag_failure".to_owned(), GateStatus::Failed),
            ("shared".to_owned(), GateStatus::Passed),
        ])
        && shared_executions == 1;
    let (cache_initial_miss, cache_hit, cache_corrupt) =
        cache_test(repository, directory, &executable, &executable_text)?;
    let cache_passed = cache_initial_miss == "not_found"
        && cache_hit == "hit"
        && cache_corrupt == "record_corrupt";
    let atomic_replacement_fail_closed = atomic_test(directory)?;
    let passed = process_passed
        && dag_passed
        && cache_passed
        && atomic_replacement_fail_closed
        && graph_rejections.values().all(|value| *value);
    Ok(SelfTestReceipt {
        contract_version: CHECK_CONTRACT_VERSION,
        kind: "checker_self_test",
        status: if passed { "passed" } else { "failed" },
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        process_statuses,
        process_reasons,
        dag_statuses,
        graph_rejections,
        shared_prerequisite_executions: shared_executions,
        cache_initial_miss,
        cache_hit,
        cache_corrupt,
        atomic_replacement_fail_closed,
        failure: (!passed).then(|| "self_test_expectation_mismatch".to_owned()),
    })
}

fn graph_rejections() -> BTreeMap<String, bool> {
    let mut cycle = Gate::new("cycle", vec!["true".to_owned()]);
    cycle.dependencies.push("cycle".to_owned());
    let mut missing = Gate::new("missing", vec!["true".to_owned()]);
    missing.dependencies.push("absent".to_owned());
    let first = Gate::new("first", vec!["true".to_owned()]);
    let second = Gate::new("second", vec!["true".to_owned()]);
    BTreeMap::from([
        ("cycle".to_owned(), GateRegistry::new(vec![cycle]).is_err()),
        (
            "duplicate_command".to_owned(),
            GateRegistry::new(vec![first, second]).is_err(),
        ),
        (
            "unknown_dependency".to_owned(),
            GateRegistry::new(vec![missing]).is_err(),
        ),
    ])
}

fn dag_test(
    repository: &Path,
    directory: &Path,
    executable: &Path,
    executable_text: &str,
) -> Result<(BTreeMap<String, GateStatus>, usize), DevError> {
    let marker = directory.join("shared.count");
    let mut shared = fixture_gate(executable_text, "shared", "append");
    shared.command.push(marker.to_string_lossy().into_owned());
    shared.identity_command = Some(vec!["$HARNESS".to_owned(), "append-shared".to_owned()]);
    let mut child_one = fixture_gate(executable_text, "child_one", "pass");
    child_one.dependencies.push("shared".to_owned());
    let mut child_two = fixture_gate(executable_text, "child_two", "pass");
    child_two.dependencies.push("shared".to_owned());
    let failure = fixture_gate(executable_text, "dag_failure", "fail");
    let mut blocked = fixture_gate(executable_text, "blocked", "pass");
    blocked.dependencies.push("dag_failure".to_owned());
    let registry = GateRegistry::new(vec![shared, child_one, child_two, failure, blocked])?;
    let selected = registry.closure(&[
        "child_one".to_owned(),
        "child_two".to_owned(),
        "blocked".to_owned(),
    ])?;
    let run = directory.join("dag");
    fs::create_dir(&run).map_err(DevError::from)?;
    let snapshot = synthetic_snapshot();
    let runtime = synthetic_runtime(repository, executable)?;
    let cache = VerificationCache::new(repository, &directory.join("dag-cache"));
    let receipts = executor::execute_dag(
        &registry,
        &selected,
        &ExecutionOptions {
            repository,
            run_directory: &run,
            snapshot: &snapshot,
            runtime: &runtime,
            maximum_workers: 2,
            allow_reuse: false,
            fresh_reason: "self_test",
            cache: &cache,
        },
    )?;
    let statuses = receipts
        .into_iter()
        .map(|receipt| (receipt.name, receipt.status))
        .collect();
    let executions = fs::read(&marker).map(|bytes| bytes.len()).unwrap_or(0);
    Ok((statuses, executions))
}

fn cache_test(
    repository: &Path,
    directory: &Path,
    executable: &Path,
    executable_text: &str,
) -> Result<(String, String, String), DevError> {
    let gate = fixture_gate(executable_text, "cache_probe", "pass");
    let registry = GateRegistry::new(vec![gate])?;
    let selected = vec!["cache_probe".to_owned()];
    let snapshot = synthetic_snapshot();
    let runtime = synthetic_runtime(repository, executable)?;
    let cache = VerificationCache::new(repository, &directory.join("cache-fixture"));
    let first_run = directory.join("cache-first");
    fs::create_dir(&first_run).map_err(DevError::from)?;
    let mut first = executor::execute_dag(
        &registry,
        &selected,
        &ExecutionOptions {
            repository,
            run_directory: &first_run,
            snapshot: &snapshot,
            runtime: &runtime,
            maximum_workers: 1,
            allow_reuse: true,
            fresh_reason: "self_test",
            cache: &cache,
        },
    )?;
    let first_receipt = first
        .pop()
        .ok_or_else(|| DevError::infrastructure("cache fixture produced no receipt"))?;
    let initial_miss = first_receipt
        .cache
        .reason
        .clone()
        .unwrap_or_else(|| "absent".to_owned());
    let gate = registry.gate("cache_probe")?;
    cache.store(gate, &first_receipt)?;
    let second_run = directory.join("cache-second");
    fs::create_dir(&second_run).map_err(DevError::from)?;
    let mut second = executor::execute_dag(
        &registry,
        &selected,
        &ExecutionOptions {
            repository,
            run_directory: &second_run,
            snapshot: &snapshot,
            runtime: &runtime,
            maximum_workers: 1,
            allow_reuse: true,
            fresh_reason: "self_test",
            cache: &cache,
        },
    )?;
    let hit_receipt = second
        .pop()
        .ok_or_else(|| DevError::infrastructure("cache hit fixture produced no receipt"))?;
    let hit = if hit_receipt.execution == ExecutionKind::Reused {
        "hit".to_owned()
    } else {
        "unexpected_fresh".to_owned()
    };
    let record = cache.record_path(gate, &first_receipt.input_fingerprint)?;
    evidence::publish(&record, b"{not-json\n")?;
    let corrupt = cache.load(
        gate,
        &first_receipt.input_fingerprint,
        &directory.join("cache-corrupt.stdout.log"),
        &directory.join("cache-corrupt.stderr.log"),
    );
    Ok((initial_miss, hit, corrupt.reason))
}

fn atomic_test(directory: &Path) -> Result<bool, DevError> {
    let path = directory.join("atomic-fixture.json");
    evidence::publish(&path, b"old\n")?;
    let mut injected = false;
    let result = evidence::publish_with_checkpoints(&path, b"new\n", &mut |checkpoint| {
        if checkpoint == AtomicCheckpoint::BytesWritten && !injected {
            injected = true;
            return Err(DevError::infrastructure("injected atomic boundary"));
        }
        Ok(())
    });
    Ok(result.is_err() && fs::read(path).is_ok_and(|bytes| bytes == b"old\n"))
}

fn fixture_gate(executable: &str, name: &str, fixture: &str) -> Gate {
    let mut gate = Gate::new(
        name,
        vec![
            executable.to_owned(),
            "__fixture".to_owned(),
            fixture.to_owned(),
        ],
    );
    gate.identity_command = Some(vec![
        "$HARNESS".to_owned(),
        "__fixture".to_owned(),
        name.to_owned(),
    ]);
    gate.cacheable = name == "cache_probe";
    gate
}

fn synthetic_snapshot() -> InputSnapshot {
    InputSnapshot {
        digest: VerificationDigest::of(b"self-test-input"),
        git_head: "self-test-head".to_owned(),
        cargo_lock_digest: VerificationDigest::of(b"self-test-lock"),
        file_count: 0,
        total_bytes: 0,
        entries: Vec::new(),
    }
}

fn synthetic_runtime(repository: &Path, executable: &Path) -> Result<RuntimeIdentity, DevError> {
    let harness = evidence::proof(executable, evidence::relative(repository, executable))?;
    Ok(RuntimeIdentity {
        digest: VerificationDigest::of(b"self-test-runtime"),
        rustc: "self-test".to_owned(),
        cargo: "self-test".to_owned(),
        platform: PlatformIdentity {
            operating_system: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            family: std::env::consts::FAMILY.to_owned(),
            child_process_control: "linux_process_group_sigkill".to_owned(),
        },
        environment_digest: VerificationDigest::of(b"self-test-environment"),
        environment_names: Vec::new(),
        harness,
        command_executables: BTreeMap::new(),
    })
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}
