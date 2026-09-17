//! Owning-context acceptance of finalized content. Consumers authenticate this decision through
//! the producing service; its portable reader deliberately does not reconstruct old filesystems.
use super::{admission, archive, bootstrap, controller, model::*, target, verifier};
use crate::{error::DevError, evidence, process};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) const CONTRACT: &str = "lkjscript-final-candidate-acceptance-1";
#[cfg(test)]
pub(super) fn canonical_terminal_fixture(value: serde_json::Value) -> Result<Vec<u8>, DevError> {
    evidence::encode_json(&serde_json::from_value::<Terminal>(value)?)
}
#[cfg(test)]
mod tests;
const WORKLOAD: &str = "release-source+six-target-owners+two-pinned-userlands+installed-recovery-1";
const MAXIMUM_RECEIPT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Incomplete,
    CandidateAccepted,
    Failed,
    Cancelled,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Stage {
    name: String,
    process: process::ProcessObservation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    name: String,
    receipt: ArtifactIdentity,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Terminal {
    schema: SchemaIdentity,
    status: Status,
    phase: String,
    source_commit: String,
    controller_source_commit: String,
    tag: String,
    acceptance_contract: String,
    workload: String,
    target_triple: String,
    target_policy_sha256: String,
    producer: HostedContext,
    verifier: ArtifactIdentity,
    assets: Vec<ArtifactIdentity>,
    manifest_sha256: Option<Sha256Digest>,
    executable: Option<ArtifactIdentity>,
    source_gates: usize,
    target_owners: usize,
    userlands: usize,
    proofs: Vec<Proof>,
    stages: Vec<Stage>,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: Option<u128>,
    elapsed_nanoseconds: u64,
    cleanup_complete: bool,
    failure: Option<String>,
}

struct Options {
    assets: PathBuf,
    source_receipt: PathBuf,
    build_receipt: PathBuf,
    verifier_identity: PathBuf,
    evidence_root: PathBuf,
    output: PathBuf,
}

pub(super) fn command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let operation = crate::next_utf8(&mut arguments, "candidate operation")?
        .ok_or_else(|| DevError::usage("candidate requires accept"))?;
    if operation != "accept" {
        return Err(DevError::usage("candidate requires accept"));
    }
    let mut values = verifier::parse_values(
        arguments,
        &[
            "--assets",
            "--source-receipt",
            "--build-receipt",
            "--verifier-identity",
            "--evidence-root",
            "--output",
        ],
    )?;
    let mut take = |name| verifier::required(&mut values, name).map(PathBuf::from);
    let options = Options {
        assets: take("--assets")?,
        source_receipt: take("--source-receipt")?,
        build_receipt: take("--build-receipt")?,
        verifier_identity: take("--verifier-identity")?,
        evidence_root: take("--evidence-root")?,
        output: take("--output")?,
    };
    accept(&options)
}

fn accept(options: &Options) -> Result<u8, DevError> {
    let repository = super::repository_root()?;
    super::ensure_clean_checkout(&repository)?;
    super::require_absolute_output(&options.output)?;
    super::require_absolute_extraction_output(&options.evidence_root)?;
    fs::create_dir(&options.evidence_root)?;
    fs::set_permissions(&options.evidence_root, fs::Permissions::from_mode(0o700))?;
    let executable = std::env::current_exe()?.canonicalize()?;
    let source = super::command_text("git", &["rev-parse", "HEAD"], &repository, 1024)?;
    let controller = std::env::var("GITHUB_WORKFLOW_SHA").unwrap_or_else(|_| source.clone());
    super::validate_git_sha(&controller, "candidate controller source")?;
    let started = Instant::now();
    let mut terminal = Terminal {
        schema: SchemaIdentity {
            identity: "lkjscript-candidate-terminal".to_owned(),
            version: 1,
        },
        status: Status::Incomplete,
        phase: "created".to_owned(),
        source_commit: source,
        controller_source_commit: controller,
        tag: format!("v{}", lkjscript::PRODUCT_VERSION),
        acceptance_contract: CONTRACT.to_owned(),
        workload: WORKLOAD.to_owned(),
        target_triple: target::TARGET_TRIPLE.to_owned(),
        target_policy_sha256: target::policy_sha256()?,
        producer: super::hosted_context(),
        verifier: file(&executable, verifier::EXECUTABLE_NAME)?,
        assets: Vec::new(),
        manifest_sha256: None,
        executable: None,
        source_gates: 0,
        target_owners: 0,
        userlands: 0,
        proofs: Vec::new(),
        stages: Vec::new(),
        started_unix_nanoseconds: super::unix_nanoseconds()?,
        completed_unix_nanoseconds: None,
        elapsed_nanoseconds: 0,
        cleanup_complete: false,
        failure: None,
    };
    evidence::publish_json(&options.output, &terminal)?;
    let cancellation = super::transferred::Cancellation::new()?;
    let control = cancellation.control.clone();
    let result = execute(options, &repository, &executable, &mut terminal, &control);
    let joined = cancellation.finish();
    let cancelled = control.cancelled();
    let result = match (result, joined) {
        (Err(primary), Err(cleanup)) => Err(DevError::infrastructure(format!(
            "{primary}; cleanup: {cleanup}"
        ))),
        (result, Ok(())) => result,
        (Ok(()), Err(cleanup)) => Err(cleanup),
    };
    terminal.completed_unix_nanoseconds = Some(super::unix_nanoseconds()?);
    terminal.elapsed_nanoseconds = super::duration_nanoseconds(started.elapsed());
    match result {
        Ok(()) if !cancelled => {
            terminal.cleanup_complete = true;
            terminal.phase = "complete".to_owned();
            terminal.status = Status::CandidateAccepted;
            if let Err(error) = validate_terminal(&terminal) {
                terminal.status = Status::Failed;
                terminal.failure = Some(error.to_string());
            }
        }
        result => {
            terminal.status = if cancelled {
                Status::Cancelled
            } else if result
                .as_ref()
                .err()
                .is_some_and(|e| e.kind() == "unavailable")
            {
                Status::Unavailable
            } else {
                Status::Failed
            };
            terminal.failure = Some(result.err().map_or_else(
                || "acceptance cancelled".to_owned(),
                |error| error.to_string(),
            ));
        }
    }
    evidence::publish_json(&options.output, &terminal)?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "status": terminal.status, "source_commit": terminal.source_commit,
            "receipt": options.output, "phase": terminal.phase,
            "source_gates": terminal.source_gates, "target_owners": terminal.target_owners,
            "failure": terminal.failure, "elapsed_nanoseconds": terminal.elapsed_nanoseconds,
        }))?
    );
    if terminal.status == Status::CandidateAccepted {
        read_terminal(&options.output)?;
        Ok(0)
    } else {
        Ok(1)
    }
}

fn execute(
    options: &Options,
    repository: &Path,
    executable: &Path,
    terminal: &mut Terminal,
    control: &process::ProcessControl,
) -> Result<(), DevError> {
    terminal.phase = "source-admission".to_owned();
    evidence::publish_json(&options.output, terminal)?;
    terminal.source_gates = crate::check::read_release_source_receipt(
        &options.source_receipt,
        repository,
        &terminal.source_commit,
    )?;
    terminal
        .proofs
        .push(proof("release-source", &options.source_receipt)?);
    terminal.assets = assets(&options.assets)?;
    let extracted = options.evidence_root.join("extracted");
    let scratch = tempfile::Builder::new()
        .prefix("container-")
        .tempdir_in(&options.evidence_root)?;
    let admitted = archive::admit_archive(
        &options.assets.join(archive::ARCHIVE_NAME),
        &options.assets.join(archive::CHECKSUM_NAME),
        scratch.path(),
        &extracted,
        control,
    )?;
    super::validate_manifest(&admitted.manifest)?;
    bootstrap::verify(&options.assets.join(bootstrap::NAME), &admitted)?;
    let bootstrap_identity = bootstrap::identity(&admitted)?;
    require(
        terminal.assets.contains(&bootstrap_identity),
        "final bootstrap identity mismatch",
    )?;
    require(
        admitted.manifest.is_publication_neutral()
            && admitted.manifest.source.tagged_commit_sha == terminal.source_commit
            && admitted.manifest.source.expected_release_tag == terminal.tag,
        "final candidate content differs from accepted source or neutral encoding",
    )?;
    terminal.manifest_sha256 = Some(admitted.manifest_sha256.clone());
    terminal.executable = Some(file(&extracted.join("lkjscript"), "lkjscript")?);
    let (lock_sha, _) = archive::sha256_file(&repository.join("Cargo.lock"))?;
    let commit_timestamp = super::command_text(
        "git",
        &["show", "-s", "--format=%ct", &terminal.source_commit],
        repository,
        1024,
    )?
    .parse::<u64>()
    .map_err(|_| DevError::corrupt("accepted source timestamp is not an unsigned integer"))?;
    let (license_sha, license_bytes) = archive::sha256_file(&repository.join("LICENSE"))?;
    let build = target::read_build_receipt(&options.build_receipt, &extracted.join("lkjscript"))?;
    require(
        build.source_commit == terminal.source_commit
            && admitted.manifest.source.commit_timestamp_unix_seconds == commit_timestamp
            && admitted.manifest.cargo_lock_sha256 == lock_sha
            && admitted.manifest.root_license.sha256 == license_sha
            && admitted.manifest.root_license.byte_length == license_bytes
            && admitted.manifest.toolchain.rustc == build.rustc
            && admitted.manifest.toolchain.cargo == build.cargo,
        "final content source, license, lockfile or build toolchain binding differs",
    )?;
    super::audit_notice(&extracted.join("THIRD-PARTY-LICENSES.html"))?;
    // A deterministic package comparison uses finalized payloads, without rebuilding the
    // executable, regenerating notices or modifying any selected public asset.
    let payload = scratch.path().join("payload");
    fs::create_dir(&payload)?;
    archive::stage_payload(
        &payload.join("lkjscript"),
        &extracted.join("lkjscript"),
        &extracted.join("LICENSE"),
        &extracted.join("THIRD-PARTY-LICENSES.html"),
        &archive::canonical_json(&admitted.manifest)?,
    )?;
    let rebuilt = archive::create_archive(
        &payload,
        &scratch.path().join(target::TAR_NAME),
        admitted.source_timestamp_unix_seconds,
    )?;
    super::require_equal_files(
        &rebuilt,
        &options.assets.join(archive::ARCHIVE_NAME),
        "final deterministic package comparison",
    )?;
    scratch.close()?;
    verifier::validate_handoff(
        executable,
        &options.verifier_identity,
        &terminal.tag,
        &terminal.source_commit,
        terminal.verifier.sha256.as_str(),
        terminal.verifier.byte_length,
    )?;
    let candidate = extracted.join("lkjscript");
    let target_root = options.evidence_root.join("target");
    let target_result = stage(
        options,
        repository,
        executable,
        terminal,
        control,
        "target-admission",
        vec![
            "release".into(),
            "admit".into(),
            "--candidate".into(),
            display(&candidate),
            "--build-receipt".into(),
            display(&options.build_receipt),
            "--evidence-root".into(),
            display(&target_root),
        ],
        Duration::from_secs(4 * 60 * 60),
    );
    // A killed verifier can leave an independently owned Docker container. Join only
    // the exact resources whose ownership was persisted before creation, on every exit.
    let target_cleanup = admission::cleanup_owned_resources(&target_root);
    match (target_result, target_cleanup) {
        (Ok(()), Ok(())) => {}
        (Err(primary), Err(cleanup)) => {
            return Err(DevError::infrastructure(format!(
                "{primary}; owned target cleanup: {cleanup}"
            )));
        }
        (Err(error), Ok(())) | (Ok(()), Err(error)) => return Err(error),
    }
    let target_receipt = target_root.join("receipt.json");
    admission::read_receipt(&target_receipt, &candidate, &terminal.source_commit)?;
    terminal.target_owners = 6;
    terminal.userlands = 2;
    terminal
        .proofs
        .push(proof("final-target", &target_receipt)?);
    let latest = options.evidence_root.join("latest-assets");
    fs::create_dir(&latest)?;
    for identity in &terminal.assets {
        archive::copy_new(
            &options.assets.join(&identity.name),
            &latest.join(&identity.name),
            0o644,
        )?;
    }
    let installation_root = options.evidence_root.join("installation");
    let arguments = |operation: &str| {
        vec![
            "release".into(),
            "transferred".into(),
            operation.into(),
            "--exact-assets".into(),
            display(&options.assets),
            "--latest-assets".into(),
            display(&latest),
            "--tag".into(),
            terminal.tag.clone(),
            "--commit".into(),
            terminal.source_commit.clone(),
            "--acquisition".into(),
            "simulated".into(),
            "--evidence-root".into(),
            display(&installation_root),
            "--verifier-identity".into(),
            display(&options.verifier_identity),
            "--expected-verifier-sha256".into(),
            terminal.verifier.sha256.as_str().to_owned(),
            "--expected-verifier-bytes".into(),
            terminal.verifier.byte_length.to_string(),
        ]
    };
    let run_arguments = arguments("installation-run");
    let verify_arguments = arguments("installation-verify");
    let installation_result = stage(
        options,
        repository,
        executable,
        terminal,
        control,
        "installation",
        run_arguments,
        Duration::from_secs(3600),
    );
    let mut installation_failures = Vec::new();
    if let Err(error) = installation_result {
        installation_failures.push(error.to_string());
    }
    for route in ["exact", "latest"] {
        if let Err(error) = admission::cleanup_owned_resources(&installation_root.join(route)) {
            installation_failures.push(error.to_string());
        }
    }
    require(
        installation_failures.is_empty(),
        &format!(
            "installation or owned cleanup failed: {}",
            installation_failures.join("; ")
        ),
    )?;
    stage(
        options,
        repository,
        executable,
        terminal,
        control,
        "installation-reader",
        verify_arguments,
        Duration::from_secs(1200),
    )?;
    terminal.proofs.push(proof(
        "installation",
        &installation_root.join("receipt.json"),
    )?);
    require(
        terminal.assets == assets(&options.assets)?
            && terminal.executable == Some(file(&candidate, "lkjscript")?)
            && terminal.verifier == file(executable, verifier::EXECUTABLE_NAME)?
            && !control.cancelled(),
        "accepted inputs changed or acceptance cancelled",
    )?;
    // Source and final-candidate readers ran in their owning context, with original logs and modes.
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn stage(
    options: &Options,
    repository: &Path,
    executable: &Path,
    terminal: &mut Terminal,
    control: &process::ProcessControl,
    name: &str,
    arguments: Vec<String>,
    timeout: Duration,
) -> Result<(), DevError> {
    terminal.phase = name.to_owned();
    evidence::publish_json(&options.output, terminal)?;
    let mut environment = process::environment();
    for name in [
        "GITHUB_ACTIONS",
        "GITHUB_REPOSITORY",
        "GITHUB_WORKFLOW",
        "GITHUB_JOB",
        "GITHUB_RUN_ID",
        "GITHUB_RUN_ATTEMPT",
        "GITHUB_SERVER_URL",
        "RUNNER_OS",
        "RUNNER_ARCH",
        "ImageOS",
        "ImageVersion",
    ] {
        if let Ok(value) = std::env::var(name) {
            environment.insert(name.to_owned(), value);
        }
    }
    let mut command = vec![display(executable)];
    command.extend(arguments);
    let observed = process::run_supervised(
        &process::ProcessSpec {
            command,
            cwd: repository.to_path_buf(),
            environment,
            timeout,
            maximum_stdout_bytes: 16 * 1024 * 1024,
            maximum_stderr_bytes: 16 * 1024 * 1024,
            stdout_path: options.evidence_root.join(format!("{name}.stdout.log")),
            stderr_path: options.evidence_root.join(format!("{name}.stderr.log")),
            unavailable_exit_code: None,
        },
        &options.evidence_root,
        Some(control),
    );
    let passed = passed(&observed);
    terminal.stages.push(Stage {
        name: name.to_owned(),
        process: observed,
    });
    evidence::publish_json(&options.output, terminal)?;
    require(
        passed && !control.cancelled(),
        &format!("{name} did not complete with joined success; retained stage logs"),
    )
}

fn passed(p: &process::ProcessObservation) -> bool {
    p.status == process::ProcessStatus::Passed
        && p.exit_code == Some(0)
        && p.signal.is_none()
        && p.reason.is_none()
        && !p.stdout_limit_exhausted
        && !p.stderr_limit_exhausted
}
fn display(path: &Path) -> String {
    path.display().to_string()
}
fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}
fn file(path: &Path, name: &str) -> Result<ArtifactIdentity, DevError> {
    let (sha256, byte_length) = archive::sha256_file(path)?;
    require(byte_length > 0, "empty candidate evidence")?;
    Ok(ArtifactIdentity {
        name: name.to_owned(),
        byte_length,
        sha256,
    })
}
fn proof(name: &str, path: &Path) -> Result<Proof, DevError> {
    Ok(Proof {
        name: name.to_owned(),
        receipt: file(path, "receipt.json")?,
    })
}
fn assets(root: &Path) -> Result<Vec<ArtifactIdentity>, DevError> {
    archive::ensure_directory(root, "candidate assets")?;
    let mut actual = fs::read_dir(root)?
        .take(4)
        .map(|entry| entry.map(|e| e.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    actual.sort();
    let mut expected = [
        archive::ARCHIVE_NAME,
        archive::CHECKSUM_NAME,
        bootstrap::NAME,
    ]
    .map(OsString::from);
    expected.sort();
    require(
        actual == expected,
        "candidate inventory must contain exactly the three finalized assets",
    )?;
    for (name, maximum) in [
        (
            archive::ARCHIVE_NAME,
            lkjscript::release_container::MAXIMUM_COMPRESSED_BYTES,
        ),
        (archive::CHECKSUM_NAME, 1024),
        (bootstrap::NAME, bootstrap::MAXIMUM_BYTES),
    ] {
        let length = archive::ensure_regular(&root.join(name), "candidate asset")?.len();
        require(
            length > 0 && length <= maximum,
            "candidate asset is empty or exceeds its native admission bound",
        )?;
    }
    [
        archive::ARCHIVE_NAME,
        archive::CHECKSUM_NAME,
        bootstrap::NAME,
    ]
    .into_iter()
    .map(|name| file(&root.join(name), name))
    .collect()
}
fn read_terminal(path: &Path) -> Result<Terminal, DevError> {
    archive::ensure_regular(path, "candidate terminal")?;
    let bytes = process::read_bounded(path, MAXIMUM_RECEIPT_BYTES)?;
    let terminal: Terminal = serde_json::from_slice(&bytes)?;
    require(
        evidence::encode_json(&terminal)? == bytes,
        "noncanonical candidate terminal",
    )?;
    validate_terminal(&terminal)?;
    Ok(terminal)
}
fn validate_terminal(t: &Terminal) -> Result<(), DevError> {
    require(
        t.schema.identity == "lkjscript-candidate-terminal"
            && t.schema.version == 1
            && t.status == Status::CandidateAccepted
            && t.phase == "complete"
            && t.failure.is_none()
            && t.acceptance_contract == CONTRACT
            && t.workload == WORKLOAD
            && t.target_triple == target::TARGET_TRIPLE
            && t.target_policy_sha256 == target::policy_sha256()?
            && t.source_gates == crate::check::RELEASE_SOURCE_GATE_COUNT
            && t.target_owners == 6
            && t.userlands == 2
            && t.cleanup_complete
            && t.started_unix_nanoseconds > 0
            && t.completed_unix_nanoseconds
                .is_some_and(|end| end >= t.started_unix_nanoseconds)
            && t.manifest_sha256.is_some()
            && t.executable
                .as_ref()
                .is_some_and(|e| e.name == "lkjscript" && e.byte_length > 0)
            && t.verifier.name == verifier::EXECUTABLE_NAME
            && t.verifier.byte_length > 0,
        "candidate terminal is incomplete, failed or bound to a different contract",
    )?;
    super::validate_git_sha(&t.source_commit, "terminal product source")?;
    super::validate_git_sha(&t.controller_source_commit, "terminal controller source")?;
    verifier::validate_tag(&t.tag)?;
    require(
        t.assets.iter().map(|a| a.name.as_str()).eq([
            archive::ARCHIVE_NAME,
            archive::CHECKSUM_NAME,
            bootstrap::NAME,
        ]) && t.assets.iter().all(|a| a.byte_length > 0)
            && t.proofs.iter().map(|p| p.name.as_str()).eq([
                "release-source",
                "final-target",
                "installation",
            ])
            && t.proofs.iter().all(|p| p.receipt.byte_length > 0)
            && t.stages.iter().map(|s| s.name.as_str()).eq([
                "target-admission",
                "installation",
                "installation-reader",
            ])
            && t.stages.iter().all(|s| passed(&s.process)),
        "candidate terminal required evidence inventory or outcome differs",
    )
}

pub(super) fn controller_content(
    path: &Path,
    source: &str,
    verifier_sha: &str,
    producer_run: u64,
    producer_attempt: u64,
) -> Result<controller::CandidateContent, DevError> {
    let t = read_terminal(path)?;
    require(
        t.source_commit == source
            && t.controller_source_commit == source
            && t.verifier.sha256.as_str() == verifier_sha
            && t.producer.github_actions.as_deref() == Some("true")
            && t.producer.repository.as_deref() == Some(super::REPOSITORY_IDENTITY)
            && t.producer.workflow.as_deref() == Some("Release")
            && t.producer.job.as_deref() == Some("candidate")
            && t.producer.runner_os.as_deref() == Some("Linux")
            && t.producer.runner_architecture.as_deref() == Some("X64")
            && t.producer.runner_image_os.as_deref() == Some("ubuntu24")
            && t.producer
                .runner_image_version
                .as_ref()
                .is_some_and(|v| !v.is_empty())
            && t.producer.run_url.as_deref()
                == Some(
                    format!(
                        "https://github.com/{}/actions/runs/{producer_run}",
                        super::REPOSITORY_IDENTITY
                    )
                    .as_str(),
                )
            && t.producer.run_id.as_deref() == Some(producer_run.to_string().as_str())
            && t.producer.run_attempt.as_deref() == Some(producer_attempt.to_string().as_str()),
        "authenticated producer identity differs from candidate terminal",
    )?;
    let convert = |a: ArtifactIdentity| controller::FileIdentity {
        name: a.name,
        sha256: a.sha256.as_str().to_owned(),
        byte_length: a.byte_length,
    };
    Ok(controller::CandidateContent {
        source_commit: t.source_commit,
        tag: t.tag,
        verifier: convert(t.verifier),
        assets: t.assets.into_iter().map(convert).collect(),
        acceptance_contract: t.acceptance_contract,
        target_policy_sha256: t.target_policy_sha256,
    })
}

pub(super) fn verify_assets_receipt(
    path: &Path,
    root: &Path,
    admitted: &archive::VerifiedArchive,
) -> Result<(), DevError> {
    let t = read_terminal(path)?;
    require(
        t.assets == assets(root)?
            && t.source_commit == admitted.manifest.source.tagged_commit_sha
            && t.tag == admitted.manifest.source.expected_release_tag
            && t.manifest_sha256.as_ref() == Some(&admitted.manifest_sha256)
            && t.executable.as_ref().is_some_and(|e| {
                e.sha256 == admitted.manifest.executable.sha256
                    && e.byte_length == admitted.manifest.executable.byte_length
            }),
        "terminal assets/content binding mismatch",
    )
}
