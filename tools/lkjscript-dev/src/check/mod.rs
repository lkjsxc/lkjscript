mod cache;
mod executor;
mod model;
mod registry;
mod self_test;
mod snapshot;

use crate::error::DevError;
use crate::evidence::{self, PublishedEvidence, VerificationDigest};
use fs2::FileExt;
use model::{
    AggregateStatus, CHECK_CONTRACT_VERSION, CacheWriteStatus, CheckReceipt, ExecutionKind,
    FailureSummary, GateStatus, InputManifest, MAXIMUM_WORKERS,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAXIMUM_RETAINED_RUNS: usize = 8;
static RUN_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
struct Options {
    profile: String,
    machine: bool,
    details: bool,
    fresh: bool,
    jobs: usize,
}

struct RunDirectory {
    path: PathBuf,
    _lease: File,
}

impl RunDirectory {
    fn path(&self) -> &Path {
        &self.path
    }
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse(arguments)?;
    if options.profile == "self-test" {
        if options.fresh || options.details || options.jobs != default_workers() {
            return Err(DevError::usage(
                "check self-test accepts only the optional --machine flag",
            ));
        }
        return self_test::run(options.machine);
    }

    let repository = repository_root()?;
    let run = new_run_directory(&repository, "")?;
    let run_directory = run.path();
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let receipt = match run_profile(&repository, run_directory, &options, started_wall, started) {
        Ok(receipt) => receipt,
        Err(error) => failure_receipt(&options, started_wall, started, error),
    };
    let receipt_path = run_directory.join("receipt.json");
    let published = evidence::publish_json(&receipt_path, &receipt)?;
    print_summary(
        &repository,
        &receipt,
        &published,
        options.machine,
        options.details,
    )?;
    Ok(if receipt.status == AggregateStatus::Passed {
        0
    } else {
        1
    })
}

pub(crate) fn fixture(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let action = crate::next_utf8(&mut arguments, "fixture action")?
        .ok_or_else(|| DevError::usage("fixture action is required"))?;
    match action.as_str() {
        "pass" => {
            io::stdout().write_all(b"out").map_err(DevError::from)?;
            io::stderr().write_all(b"err").map_err(DevError::from)?;
            Ok(0)
        }
        "fail" => Ok(7),
        "sleep" => {
            std::thread::sleep(Duration::from_secs(5));
            Ok(0)
        }
        "stdout" => {
            write_repeated(&mut io::stdout(), b'x', 4_096)?;
            Ok(0)
        }
        "stderr" => {
            write_repeated(&mut io::stderr(), b'y', 4_096)?;
            Ok(0)
        }
        "append" => {
            let path = crate::next_utf8(&mut arguments, "marker path")?
                .ok_or_else(|| DevError::usage("append fixture requires a marker path"))?;
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|error| {
                    DevError::infrastructure(format!("open fixture marker '{path}': {error}"))
                })?;
            file.write_all(b"x").map_err(|error| {
                DevError::infrastructure(format!("write fixture marker '{path}': {error}"))
            })?;
            file.sync_all().map_err(|error| {
                DevError::infrastructure(format!("synchronize fixture marker '{path}': {error}"))
            })?;
            Ok(0)
        }
        value => Err(DevError::usage(format!("unknown fixture action '{value}'"))),
    }
}

fn run_profile(
    repository: &Path,
    run_directory: &Path,
    options: &Options,
    started_wall: u128,
    started: Instant,
) -> Result<CheckReceipt, DevError> {
    rotate_runs(repository, run_directory, "")?;
    let harness_copy = copy_harness(run_directory)?;
    let registry = registry::base_registry(repository, run_directory, &harness_copy)?;
    let requested = if options.profile == "changed" {
        snapshot::changed_profile(repository)?
    } else {
        registry::profile(&options.profile).ok_or_else(|| {
            DevError::usage(format!("unknown check profile '{}'", options.profile))
        })?
    };
    let selected = registry.closure(&requested)?;
    let profile_digest = registry.profile_digest(&options.profile, &requested)?;
    let initial = snapshot::capture(repository)?;
    let inputs_path = run_directory.join("inputs.json");
    evidence::publish_json(
        &inputs_path,
        &InputManifest {
            contract_version: CHECK_CONTRACT_VERSION,
            snapshot: initial.clone(),
        },
    )?;
    let command_names = selected
        .iter()
        .filter_map(|name| registry.gate(name).ok())
        .filter_map(|gate| {
            gate.command.first().cloned().map(|command| {
                let identity = gate
                    .identity_command()
                    .first()
                    .cloned()
                    .unwrap_or_else(|| command.clone());
                (command, identity)
            })
        })
        .collect::<BTreeSet<_>>();
    let runtime = snapshot::runtime_identity(repository, command_names)?;
    let dag_path = run_directory.join("dag.json");
    evidence::publish_json(
        &dag_path,
        &registry.manifest(&requested, &selected, options.jobs)?,
    )?;
    let fresh_required = options.profile == "full" || options.fresh;
    let fresh_reason = if options.profile == "full" {
        "full_profile_requires_fresh"
    } else if options.fresh {
        "explicit_fresh"
    } else {
        "exact_input_match"
    };
    let cache = cache::VerificationCache::new(
        repository,
        &repository.join(".artifacts/lkjscript-dev/check/cache"),
    );
    let mut gates = executor::execute_dag(
        &registry,
        &selected,
        &executor::ExecutionOptions {
            repository,
            run_directory,
            snapshot: &initial,
            runtime: &runtime,
            maximum_workers: options.jobs,
            allow_reuse: !fresh_required,
            fresh_reason,
            cache: &cache,
        },
    )?;

    let (final_digest, input_stable, final_error) = match snapshot::capture(repository) {
        Ok(final_snapshot) => {
            let stable = final_snapshot.digest == initial.digest;
            (Some(final_snapshot.digest), stable, None)
        }
        Err(error) => (None, false, Some(error)),
    };
    if input_stable {
        for receipt in &mut gates {
            if receipt.status == GateStatus::Passed && receipt.execution == ExecutionKind::Fresh {
                let gate = registry.gate(&receipt.name)?;
                match cache.store(gate, receipt) {
                    Ok(record) => {
                        receipt.cache.write = Some(CacheWriteStatus::Stored);
                        receipt.cache.record = Some(record);
                    }
                    Err(error) => {
                        receipt.cache.write = Some(CacheWriteStatus::Failed);
                        receipt.cache.reason = Some(format!("cache_store:{}", error.kind()));
                    }
                }
            }
        }
    } else {
        for receipt in &mut gates {
            if receipt.status == GateStatus::Passed && receipt.execution == ExecutionKind::Fresh {
                receipt.cache.write = Some(CacheWriteStatus::WithheldInputChanged);
            }
        }
    }

    let passed_gates = gates.iter().filter(|gate| gate.passed()).count();
    let fresh_passed_gates = gates
        .iter()
        .filter(|gate| gate.passed() && gate.execution == ExecutionKind::Fresh)
        .count();
    let reused_passed_gates = gates
        .iter()
        .filter(|gate| gate.passed() && gate.execution == ExecutionKind::Reused)
        .count();
    let unrun_gates = gates
        .iter()
        .filter(|gate| gate.status == GateStatus::Skipped)
        .map(|gate| gate.name.clone())
        .collect::<Vec<_>>();
    let all_passed = passed_gates == gates.len();
    let full_is_fresh = options.profile != "full" || reused_passed_gates == 0;
    let status = if all_passed && input_stable && full_is_fresh && final_error.is_none() {
        AggregateStatus::Passed
    } else {
        AggregateStatus::Failed
    };
    let failure = if let Some(gate) = gates
        .iter()
        .find(|gate| gate.status != GateStatus::Passed && gate.status != GateStatus::Skipped)
    {
        Some(FailureSummary {
            owner: gate.name.clone(),
            status: gate_status_name(gate.status).to_owned(),
            reason: gate.reason.clone().unwrap_or_else(|| "unknown".to_owned()),
        })
    } else if let Some(error) = final_error {
        Some(FailureSummary {
            owner: "input_stability".to_owned(),
            status: error.kind().to_owned(),
            reason: error.message().to_owned(),
        })
    } else if !input_stable {
        Some(FailureSummary {
            owner: "input_stability".to_owned(),
            status: "failed".to_owned(),
            reason: "worktree_changed_during_run".to_owned(),
        })
    } else if !full_is_fresh {
        Some(FailureSummary {
            owner: "fresh_policy".to_owned(),
            status: "failed".to_owned(),
            reason: "full_profile_reused_evidence".to_owned(),
        })
    } else {
        None
    };
    Ok(CheckReceipt {
        contract_version: CHECK_CONTRACT_VERSION,
        status,
        profile: options.profile.clone(),
        profile_definition_digest: profile_digest,
        started_unix_nanoseconds: started_wall,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        git_head: Some(initial.git_head.clone()),
        worktree_input_digest: Some(initial.digest.clone()),
        final_worktree_input_digest: final_digest,
        input_stable,
        input_manifest: Some(evidence::relative(repository, &inputs_path)),
        dag_manifest: Some(evidence::relative(repository, &dag_path)),
        runtime: Some(runtime),
        requested_gates: requested,
        selected_gates: selected,
        passed_gates,
        fresh_passed_gates,
        reused_passed_gates,
        unrun_gates,
        maximum_workers: options.jobs,
        fresh_required,
        gates,
        failure,
    })
}

fn failure_receipt(
    options: &Options,
    started_wall: u128,
    started: Instant,
    error: DevError,
) -> CheckReceipt {
    CheckReceipt {
        contract_version: CHECK_CONTRACT_VERSION,
        status: AggregateStatus::Failed,
        profile: options.profile.clone(),
        profile_definition_digest: VerificationDigest::of(options.profile.as_bytes()),
        started_unix_nanoseconds: started_wall,
        completed_unix_nanoseconds: unix_nanoseconds().unwrap_or(started_wall),
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        git_head: None,
        worktree_input_digest: None,
        final_worktree_input_digest: None,
        input_stable: false,
        input_manifest: None,
        dag_manifest: None,
        runtime: None,
        requested_gates: Vec::new(),
        selected_gates: Vec::new(),
        passed_gates: 0,
        fresh_passed_gates: 0,
        reused_passed_gates: 0,
        unrun_gates: Vec::new(),
        maximum_workers: options.jobs,
        fresh_required: options.fresh || options.profile == "full",
        gates: Vec::new(),
        failure: Some(FailureSummary {
            owner: "harness".to_owned(),
            status: error.kind().to_owned(),
            reason: error.message().to_owned(),
        }),
    }
}

fn print_summary(
    repository: &Path,
    receipt: &CheckReceipt,
    published: &PublishedEvidence,
    machine: bool,
    details: bool,
) -> Result<(), DevError> {
    #[derive(Serialize)]
    struct Summary<'a> {
        contract_version: u32,
        status: &'static str,
        profile: &'a str,
        passed: usize,
        fresh: usize,
        reused: usize,
        selected: usize,
        elapsed_nanoseconds: u64,
        receipt: String,
        receipt_bytes: u64,
        receipt_digest: &'a VerificationDigest,
        failure: &'a Option<FailureSummary>,
    }
    let summary = Summary {
        contract_version: receipt.contract_version,
        status: aggregate_status_name(receipt.status),
        profile: &receipt.profile,
        passed: receipt.passed_gates,
        fresh: receipt.fresh_passed_gates,
        reused: receipt.reused_passed_gates,
        selected: receipt.selected_gates.len(),
        elapsed_nanoseconds: receipt.elapsed_nanoseconds,
        receipt: evidence::relative(repository, &published.path),
        receipt_bytes: published.bytes,
        receipt_digest: &published.digest,
        failure: &receipt.failure,
    };
    if machine {
        let line = serde_json::to_string(&summary).map_err(|error| {
            DevError::infrastructure(format!("encode compact check summary: {error}"))
        })?;
        println!("{line}");
        return Ok(());
    }
    println!(
        "check {}: profile={} passed={}/{} fresh={} reused={} elapsed={:.3}s receipt={} digest={}",
        summary.status,
        summary.profile,
        summary.passed,
        summary.selected,
        summary.fresh,
        summary.reused,
        summary.elapsed_nanoseconds as f64 / 1_000_000_000.0,
        summary.receipt,
        summary.receipt_digest,
    );
    if let Some(failure) = &receipt.failure {
        println!(
            "failure: owner={} status={} reason={}",
            failure.owner, failure.status, failure.reason
        );
    } else if details {
        for gate in &receipt.gates {
            println!(
                "gate {}: {} execution={} elapsed={:.3}s",
                gate.name,
                gate_status_name(gate.status),
                execution_name(gate.execution),
                gate.elapsed_nanoseconds as f64 / 1_000_000_000.0,
            );
        }
    }
    Ok(())
}

fn parse(mut arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let profile = crate::next_utf8(&mut arguments, "check profile")?
        .ok_or_else(|| DevError::usage("check profile is required"))?;
    let mut machine = false;
    let mut details = false;
    let mut fresh = false;
    let mut jobs = default_workers();
    while let Some(argument) = crate::next_utf8(&mut arguments, "check option")? {
        match argument.as_str() {
            "--machine" if !machine => machine = true,
            "--details" if !details => details = true,
            "--fresh" if !fresh => fresh = true,
            "--jobs" => {
                let value = crate::next_utf8(&mut arguments, "worker count")?
                    .ok_or_else(|| DevError::usage("--jobs requires a value"))?;
                jobs = value
                    .parse::<usize>()
                    .map_err(|_| DevError::usage("--jobs must be an integer"))?;
            }
            value => {
                return Err(DevError::usage(format!(
                    "unknown or duplicate option '{value}'"
                )));
            }
        }
    }
    if machine && details {
        return Err(DevError::usage("--machine and --details are incompatible"));
    }
    if jobs == 0 || jobs > MAXIMUM_WORKERS {
        return Err(DevError::usage(format!(
            "--jobs must be between 1 and {MAXIMUM_WORKERS}"
        )));
    }
    if profile != "self-test" && profile != "changed" && registry::profile(&profile).is_none() {
        return Err(DevError::usage(format!(
            "unknown check profile '{profile}'"
        )));
    }
    Ok(Options {
        profile,
        machine,
        details,
        fresh,
        jobs,
    })
}

fn repository_root() -> Result<PathBuf, DevError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| DevError::infrastructure("resolve repository root"))
}

fn new_run_directory(repository: &Path, prefix: &str) -> Result<RunDirectory, DevError> {
    let root = repository.join(".artifacts/lkjscript-dev/check");
    fs::create_dir_all(&root).map_err(|error| {
        DevError::infrastructure(format!(
            "create check evidence root '{}': {error}",
            root.display()
        ))
    })?;
    let ordinal = RUN_ORDINAL.fetch_add(1, Ordering::Relaxed);
    let name = format!(
        "{prefix}{}-{}-{ordinal}",
        unix_nanoseconds()?,
        std::process::id()
    );
    let directory = root.join(name);
    fs::create_dir(&directory).map_err(|error| {
        DevError::infrastructure(format!(
            "create check run '{}': {error}",
            directory.display()
        ))
    })?;
    let lease_path = directory.join("ACTIVE.lock");
    let lease = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&lease_path)
        .map_err(|error| {
            DevError::infrastructure(format!(
                "create check run lease '{}': {error}",
                lease_path.display()
            ))
        })?;
    lease.lock_exclusive().map_err(|error| {
        DevError::infrastructure(format!(
            "lock check run lease '{}': {error}",
            lease_path.display()
        ))
    })?;
    Ok(RunDirectory {
        path: directory,
        _lease: lease,
    })
}

fn rotate_runs(repository: &Path, current: &Path, prefix: &str) -> Result<(), DevError> {
    rotate_runs_with_limit(repository, current, prefix, MAXIMUM_RETAINED_RUNS)
}

fn rotate_runs_with_limit(
    repository: &Path,
    current: &Path,
    prefix: &str,
    maximum_retained: usize,
) -> Result<(), DevError> {
    let root = repository.join(".artifacts/lkjscript-dev/check");
    let mut runs = Vec::new();
    let mut managed_count = 0_usize;
    for entry in fs::read_dir(&root).map_err(|error| {
        DevError::infrastructure(format!(
            "read check evidence root '{}': {error}",
            root.display()
        ))
    })? {
        let entry = entry.map_err(|error| {
            DevError::infrastructure(format!("read check evidence entry: {error}"))
        })?;
        let path = entry.path();
        if !managed_run(&path, prefix)? {
            continue;
        }
        managed_count = managed_count.saturating_add(1);
        if path == current || run_is_active(&path)? {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH);
        runs.push((modified, path));
    }
    runs.sort_by_key(|(modified, _)| *modified);
    let remove = managed_count
        .saturating_sub(maximum_retained)
        .min(runs.len());
    for (_, path) in runs.into_iter().take(remove) {
        fs::remove_dir_all(&path).map_err(|error| {
            DevError::infrastructure(format!(
                "remove old check evidence '{}': {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

fn run_is_active(path: &Path) -> Result<bool, DevError> {
    let lease_path = path.join("ACTIVE.lock");
    let metadata = match fs::symlink_metadata(&lease_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "inspect check run lease '{}': {error}",
                lease_path.display()
            )));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::infrastructure(format!(
            "check run lease '{}' is not a regular file",
            lease_path.display()
        )));
    }
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lease_path)
        .map_err(|error| {
            DevError::infrastructure(format!(
                "open check run lease '{}': {error}",
                lease_path.display()
            ))
        })?;
    match lease.try_lock_exclusive() {
        Ok(()) => {
            FileExt::unlock(&lease).map_err(|error| {
                DevError::infrastructure(format!(
                    "unlock check run lease '{}': {error}",
                    lease_path.display()
                ))
            })?;
            Ok(false)
        }
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(true),
        Err(error) => Err(DevError::infrastructure(format!(
            "probe check run lease '{}': {error}",
            lease_path.display()
        ))),
    }
}

fn managed_run(path: &Path, prefix: &str) -> Result<bool, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect check evidence '{}': {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(false);
    }
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return Ok(false);
    };
    let Some(suffix) = name.strip_prefix(prefix) else {
        return Ok(false);
    };
    let fields = suffix.split('-').collect::<Vec<_>>();
    Ok(fields.len() == 3
        && fields
            .iter()
            .all(|field| !field.is_empty() && field.bytes().all(|byte| byte.is_ascii_digit())))
}

fn copy_harness(run_directory: &Path) -> Result<PathBuf, DevError> {
    let source = std::env::current_exe()
        .map_err(|error| DevError::infrastructure(format!("resolve running harness: {error}")))?;
    let destination = run_directory.join("lkjscript-dev");
    fs::copy(&source, &destination).map_err(|error| {
        DevError::infrastructure(format!(
            "copy harness '{}' to '{}': {error}",
            source.display(),
            destination.display()
        ))
    })?;
    evidence::synchronize_file(&destination)?;
    Ok(destination)
}

fn write_repeated(output: &mut impl Write, byte: u8, count: usize) -> Result<(), DevError> {
    let bytes = vec![byte; count];
    output.write_all(&bytes).map_err(DevError::from)
}

fn default_workers() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .clamp(1, 4)
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock before epoch: {error}")))
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn aggregate_status_name(status: AggregateStatus) -> &'static str {
    match status {
        AggregateStatus::Passed => "passed",
        AggregateStatus::Failed => "failed",
    }
}

fn gate_status_name(status: GateStatus) -> &'static str {
    match status {
        GateStatus::Passed => "passed",
        GateStatus::Failed => "failed",
        GateStatus::Unavailable => "unavailable",
        GateStatus::Timeout => "timeout",
        GateStatus::OutputExhausted => "output_exhausted",
        GateStatus::Signaled => "signaled",
        GateStatus::InfrastructureFailure => "infrastructure_failure",
        GateStatus::Skipped => "skipped",
    }
}

fn execution_name(execution: ExecutionKind) -> &'static str {
    match execution {
        ExecutionKind::Fresh => "fresh",
        ExecutionKind::Reused => "reused",
        ExecutionKind::Skipped => "skipped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infrastructure_failure_still_has_an_atomic_aggregate_receipt() {
        let temporary = tempfile::tempdir().expect("temporary aggregate evidence");
        let options = Options {
            profile: "focused".to_owned(),
            machine: true,
            details: false,
            fresh: false,
            jobs: 1,
        };
        let receipt = failure_receipt(
            &options,
            1,
            Instant::now(),
            DevError::infrastructure("injected aggregate failure"),
        );
        let path = temporary.path().join("receipt.json");
        let published = evidence::publish_json(&path, &receipt).expect("publish failure receipt");
        assert!(published.bytes > 0);
        let retained: CheckReceipt = serde_json::from_slice(
            &fs::read(path).expect("read retained aggregate failure receipt"),
        )
        .expect("decode retained aggregate failure receipt");
        assert_eq!(retained.status, AggregateStatus::Failed);
        assert_eq!(
            retained.failure.expect("failure summary").reason,
            "injected aggregate failure"
        );
    }

    #[test]
    fn rotation_preserves_another_invocations_locked_run() {
        let temporary = tempfile::tempdir().expect("temporary rotation repository");
        let active = new_run_directory(temporary.path(), "").expect("active run");
        let root = temporary.path().join(".artifacts/lkjscript-dev/check");
        let completed = root.join("1-1-1");
        fs::create_dir(&completed).expect("completed run directory");
        File::create(completed.join("ACTIVE.lock")).expect("completed run lease");
        let current = new_run_directory(temporary.path(), "").expect("current run");
        rotate_runs_with_limit(temporary.path(), current.path(), "", 2)
            .expect("rotate completed runs");
        assert!(active.path().is_dir());
        assert!(current.path().is_dir());
        assert!(!completed.exists());
    }
}
