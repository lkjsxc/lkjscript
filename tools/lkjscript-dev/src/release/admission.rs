use super::archive;
use super::model::{
    EvidenceClassification, ExternalEvidence, HostedContext, SchemaIdentity,
    VerificationClassification,
};
use super::target::{self, BuiltCandidate, TargetBuildReceipt, UserlandPolicy};
use crate::distributed_http;
use crate::error::DevError;
use crate::evidence;
use crate::process::{self, ProcessObservation, ProcessSpec, ProcessStatus};
use crate::service;
use crate::stateful_http;
use lkjscript::platform::control::{CompactRecord, parse_records};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const ADMISSION_SCHEMA: &str = "lkjscript-target-admission-receipt";
const ADMISSION_SCHEMA_VERSION: u32 = 1;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const ORACLE_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const IMAGE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MAXIMUM_COMMAND_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_RECEIPT_BYTES: u64 = 128 * 1024 * 1024;
const MAXIMUM_ROOTFS_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug)]
struct Options {
    candidate: PathBuf,
    build_receipt: PathBuf,
    evidence_root: PathBuf,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AdmissionStatus {
    Passed,
    Failed,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExecutableBinding {
    path: String,
    byte_length: u64,
    mode: u32,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CommandEvidence {
    name: String,
    command: Vec<String>,
    expected: String,
    process: ProcessObservation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ImageIdentity {
    requested: String,
    image_id: String,
    repository_digests: Vec<String>,
    operating_system: String,
    architecture: String,
    virtual_size_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct UserlandObservation {
    role: String,
    status: AdmissionStatus,
    image: ImageIdentity,
    expected_libc: String,
    os_release_sha256: String,
    os_release: Vec<String>,
    rootfs_archive_bytes: u64,
    rootfs_archive_sha256: String,
    candidate_sha256: String,
    candidate_mode: u32,
    network_policy: String,
    candidate_commands: u64,
    cli_contract: u64,
    registry_digest: String,
    initial_revision: String,
    accepted_revision: String,
    artifact_bytes: u64,
    artifact_sha256: String,
    clean_artifact_sha256: String,
    execution_value: String,
    differential_equal: bool,
    container_cleanup_complete: bool,
    temporary_root_removed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OracleObservation {
    name: String,
    status: AdmissionStatus,
    receipt: ExternalEvidence,
    verifier_sha256: String,
    candidate_sha256: String,
    elapsed_nanoseconds: u64,
    commands: u64,
    runners: u64,
    requests: u64,
    cleanup_complete: bool,
    prerequisite: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HostTools {
    docker: String,
    sudo: String,
    unshare: String,
    chroot: String,
    tar: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TargetAdmissionReceipt {
    schema: SchemaIdentity,
    status: AdmissionStatus,
    source_commit: String,
    target_policy_sha256: String,
    target_triple: String,
    runtime_linkage: String,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: u128,
    elapsed_nanoseconds: u64,
    hosted_context: HostedContext,
    host_tools: HostTools,
    verifier: ExecutableBinding,
    build_receipt: ExternalEvidence,
    candidate: BuiltCandidate,
    userlands: Vec<UserlandObservation>,
    oracles: Vec<OracleObservation>,
    commands: Vec<CommandEvidence>,
    classifications: Vec<VerificationClassification>,
    cleanup_complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<String>,
}

struct Context {
    evidence_root: PathBuf,
    ordinal: u64,
    commands: Vec<CommandEvidence>,
}

enum Expected {
    Passed,
    Exit(i32),
}

pub(super) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse_options(arguments)?;
    let repository = super::repository_root()?;
    super::ensure_clean_checkout(&repository)?;
    require_absent_root(&options.evidence_root)?;
    super::require_absolute_regular_executable(&options.candidate, "target candidate")?;
    super::require_absolute_regular(&options.build_receipt, "target build receipt")?;
    let source_commit = super::command_text("git", &["rev-parse", "HEAD"], &repository, 1024)?;
    super::validate_git_sha(&source_commit, "target admission source commit")?;
    let build_receipt = target::read_build_receipt(&options.build_receipt, &options.candidate)?;
    if build_receipt.source_commit != source_commit {
        return Err(DevError::corrupt(
            "target build receipt does not select the current clean source commit",
        ));
    }
    let evidence_root = create_evidence_root(&options.evidence_root)?;
    let started = Instant::now();
    let started_unix_nanoseconds = unix_nanoseconds()?;
    let verifier_path = current_verifier()?;
    let verifier = executable_binding(&verifier_path)?;
    let candidate = target::observe_candidate(&options.candidate)?;
    let build_receipt_evidence = external_evidence(&options.build_receipt)?;
    let host_tools = host_tools(&repository)?;
    let mut context = Context {
        evidence_root: evidence_root.clone(),
        ordinal: 0,
        commands: Vec::new(),
    };
    let mut userlands = Vec::new();
    for policy in target::policy().userlands {
        userlands.push(run_userland(&mut context, &policy, &options.candidate)?);
    }
    let oracles = run_oracles(
        &mut context,
        &repository,
        &verifier_path,
        &options.candidate,
    )?;
    let classifications = vec![
        fresh(
            "static_linkage",
            "exact candidate passed first-party ELF inspection",
        ),
        fresh(
            "musl_userland",
            "complete command lifecycle passed in the pinned musl userland",
        ),
        fresh(
            "older_glibc_userland",
            "complete command lifecycle passed in the pinned older-glibc userland",
        ),
        fresh(
            "distributed_http",
            "transferred copied-binary HTTP oracle passed",
        ),
        fresh(
            "stateful_http",
            "transferred PostgreSQL-backed BBS oracle passed",
        ),
        fresh(
            "service_acceptance",
            "maintained lkjournal service oracle passed with the exact candidate",
        ),
    ];
    let receipt = TargetAdmissionReceipt {
        schema: SchemaIdentity {
            identity: ADMISSION_SCHEMA.to_owned(),
            version: ADMISSION_SCHEMA_VERSION,
        },
        status: AdmissionStatus::Passed,
        source_commit,
        target_policy_sha256: target::policy_sha256()?,
        target_triple: target::TARGET_TRIPLE.to_owned(),
        runtime_linkage: target::LINKAGE_MODEL.to_owned(),
        started_unix_nanoseconds,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        hosted_context: super::hosted_context(),
        host_tools,
        verifier,
        build_receipt: build_receipt_evidence,
        candidate,
        userlands,
        oracles,
        commands: context.commands,
        classifications,
        cleanup_complete: true,
        failure: None,
    };
    validate_receipt(&receipt, &options.candidate, &build_receipt, &verifier_path)?;
    let receipt_path = evidence_root.join("receipt.json");
    let published = evidence::publish_json(&receipt_path, &receipt)?;
    let (receipt_sha256, receipt_bytes) = archive::sha256_file(&receipt_path)?;
    if published.bytes != receipt_bytes {
        return Err(DevError::infrastructure(
            "published target admission receipt length changed",
        ));
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "status": "passed",
            "schema": receipt.schema,
            "source_commit": receipt.source_commit,
            "target_triple": receipt.target_triple,
            "runtime_linkage": receipt.runtime_linkage,
            "target_policy_sha256": receipt.target_policy_sha256,
            "candidate_sha256": receipt.candidate.sha256,
            "receipt": receipt_path,
            "receipt_bytes": receipt_bytes,
            "receipt_sha256": receipt_sha256,
            "receipt_digest": published.digest,
            "userlands": receipt.userlands.len(),
            "oracles": receipt.oracles.len(),
            "cleanup_complete": receipt.cleanup_complete,
        }))
        .map_err(|error| DevError::infrastructure(format!(
            "encode target admission summary: {error}"
        )))?
    );
    Ok(0)
}

pub(super) fn read_receipt(
    path: &Path,
    candidate: &Path,
    expected_source_commit: &str,
) -> Result<TargetAdmissionReceipt, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect target admission receipt '{}': {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAXIMUM_RECEIPT_BYTES
    {
        return Err(DevError::corrupt(
            "target admission receipt is unsafe or oversized",
        ));
    }
    let bytes = fs::read(path).map_err(|error| {
        DevError::infrastructure(format!(
            "read target admission receipt '{}': {error}",
            path.display()
        ))
    })?;
    let receipt: TargetAdmissionReceipt = serde_json::from_slice(&bytes)
        .map_err(|error| DevError::corrupt(format!("decode target admission receipt: {error}")))?;
    if evidence::encode_json(&receipt)? != bytes {
        return Err(DevError::corrupt(
            "target admission receipt is not in canonical evidence encoding",
        ));
    }
    if receipt.source_commit != expected_source_commit {
        return Err(DevError::corrupt(
            "target admission receipt selects a foreign source commit",
        ));
    }
    let build_path = Path::new(&receipt.build_receipt.path);
    let build_receipt = target::read_build_receipt(build_path, candidate)?;
    let verifier = current_verifier()?;
    validate_receipt(&receipt, candidate, &build_receipt, &verifier)?;
    if receipt.build_receipt != external_evidence(build_path)? {
        return Err(DevError::corrupt(
            "target admission build-receipt evidence changed",
        ));
    }
    Ok(receipt)
}

fn validate_receipt(
    receipt: &TargetAdmissionReceipt,
    candidate_path: &Path,
    build_receipt: &TargetBuildReceipt,
    verifier_path: &Path,
) -> Result<(), DevError> {
    let observed_candidate = target::observe_candidate(candidate_path)?;
    let observed_verifier = executable_binding(verifier_path)?;
    let policy = target::policy();
    let expected_userlands = policy
        .userlands
        .iter()
        .map(|item| (item.role.as_str(), item.image.as_str()))
        .collect::<Vec<_>>();
    let observed_userlands = receipt
        .userlands
        .iter()
        .map(|item| (item.role.as_str(), item.image.requested.as_str()))
        .collect::<Vec<_>>();
    let expected_oracles = ["distributed_http", "stateful_http", "service_acceptance"];
    let observed_oracles = receipt
        .oracles
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>();
    let expected_classifications = [
        "static_linkage",
        "musl_userland",
        "older_glibc_userland",
        "distributed_http",
        "stateful_http",
        "service_acceptance",
    ];
    let observed_classifications = receipt
        .classifications
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>();
    if receipt.schema.identity != ADMISSION_SCHEMA
        || receipt.schema.version != ADMISSION_SCHEMA_VERSION
        || receipt.status != AdmissionStatus::Passed
        || receipt.target_policy_sha256 != target::policy_sha256()?
        || receipt.target_triple != target::TARGET_TRIPLE
        || receipt.runtime_linkage != target::LINKAGE_MODEL
        || receipt.completed_unix_nanoseconds < receipt.started_unix_nanoseconds
        || receipt.verifier.path != observed_verifier.path
        || receipt.verifier.byte_length != observed_verifier.byte_length
        || receipt.verifier.mode != observed_verifier.mode
        || receipt.verifier.sha256 != observed_verifier.sha256
        || receipt.candidate.byte_length != observed_candidate.byte_length
        || receipt.candidate.mode != observed_candidate.mode
        || receipt.candidate.sha256 != observed_candidate.sha256
        || receipt.candidate.elf != observed_candidate.elf
        || build_receipt.source_commit != receipt.source_commit
        || build_receipt.target_policy_sha256 != receipt.target_policy_sha256
        || build_receipt.candidate.sha256 != receipt.candidate.sha256
        || expected_userlands != observed_userlands
        || receipt.userlands.iter().any(|item| {
            item.status != AdmissionStatus::Passed
                || !item.container_cleanup_complete
                || !item.temporary_root_removed
        })
        || expected_oracles.as_slice() != observed_oracles
        || receipt.oracles.iter().any(|item| {
            item.status != AdmissionStatus::Passed
                || item.candidate_sha256 != receipt.candidate.sha256
                || !item.cleanup_complete
        })
        || expected_classifications.as_slice() != observed_classifications
        || receipt.classifications.iter().any(|item| {
            item.classification != EvidenceClassification::FreshPassed || item.detail.is_empty()
        })
        || receipt.commands.is_empty()
        || !receipt.cleanup_complete
        || receipt.failure.is_some()
    {
        return Err(DevError::corrupt(
            "target admission receipt binding or required evidence mismatch",
        ));
    }
    Ok(())
}

fn run_userland(
    context: &mut Context,
    policy: &UserlandPolicy,
    candidate: &Path,
) -> Result<UserlandObservation, DevError> {
    context.external_success(
        &format!("{}-image-pull", policy.role),
        vec![
            "docker".to_owned(),
            "pull".to_owned(),
            "--platform".to_owned(),
            "linux/amd64".to_owned(),
            policy.image.clone(),
        ],
        IMAGE_TIMEOUT,
    )?;
    let inspect = context.external_success(
        &format!("{}-image-inspect", policy.role),
        vec![
            "docker".to_owned(),
            "image".to_owned(),
            "inspect".to_owned(),
            policy.image.clone(),
        ],
        COMMAND_TIMEOUT,
    )?;
    let image = inspect_image(&inspect, policy)?;
    let temporary = tempfile::Builder::new()
        .prefix(&format!(".{}-userland-", policy.role))
        .tempdir_in(&context.evidence_root)
        .map_err(|error| DevError::infrastructure(format!("create userland root: {error}")))?;
    let temporary_path = temporary.path().canonicalize()?;
    let rootfs_archive = temporary_path.join("rootfs.tar");
    let rootfs = temporary_path.join("rootfs");
    fs::create_dir(&rootfs)?;
    let container = format!(
        "lkjscript-admission-{}-{}-{}",
        safe_name(&policy.role)?,
        std::process::id(),
        unix_nanoseconds()?
    );
    let create = context.external_success(
        &format!("{}-container-create", policy.role),
        vec![
            "docker".to_owned(),
            "create".to_owned(),
            "--name".to_owned(),
            container.clone(),
            "--network".to_owned(),
            "host".to_owned(),
            policy.image.clone(),
            "/bin/true".to_owned(),
        ],
        COMMAND_TIMEOUT,
    );
    create?;
    let export_result = context.external_success(
        &format!("{}-container-export", policy.role),
        vec![
            "docker".to_owned(),
            "export".to_owned(),
            "--output".to_owned(),
            rootfs_archive.display().to_string(),
            container.clone(),
        ],
        IMAGE_TIMEOUT,
    );
    let cleanup = context.external_success(
        &format!("{}-container-remove", policy.role),
        vec![
            "docker".to_owned(),
            "rm".to_owned(),
            "--force".to_owned(),
            container,
        ],
        COMMAND_TIMEOUT,
    );
    export_result?;
    cleanup?;
    let rootfs_metadata = archive::ensure_regular(&rootfs_archive, "exported userland rootfs")?;
    if rootfs_metadata.len() == 0 || rootfs_metadata.len() > MAXIMUM_ROOTFS_ARCHIVE_BYTES {
        return Err(DevError::corrupt(
            "exported userland rootfs has an invalid byte length",
        ));
    }
    context.external_success(
        &format!("{}-rootfs-extract", policy.role),
        vec![
            "tar".to_owned(),
            "--extract".to_owned(),
            "--file".to_owned(),
            rootfs_archive.display().to_string(),
            "--directory".to_owned(),
            rootfs.display().to_string(),
            "--no-same-owner".to_owned(),
            "--no-same-permissions".to_owned(),
        ],
        IMAGE_TIMEOUT,
    )?;
    let (rootfs_sha256, rootfs_bytes) = archive::sha256_file(&rootfs_archive)?;
    let (os_release_sha256, os_release) = rootfs_os_release(&rootfs, policy)?;
    let work = rootfs.join("work");
    if fs::symlink_metadata(&work).is_ok() {
        return Err(DevError::corrupt(
            "pinned userland unexpectedly contains /work",
        ));
    }
    fs::create_dir(&work)?;
    let copied_candidate = work.join("lkjscript");
    archive::copy_new(candidate, &copied_candidate, 0o755)?;
    let copied = target::observe_candidate(&copied_candidate)?;
    let source = target::observe_candidate(candidate)?;
    if copied.sha256 != source.sha256 || copied.byte_length != source.byte_length {
        return Err(DevError::corrupt(
            "userland candidate copy disagrees with the exact target candidate",
        ));
    }

    let mut candidate_commands = 0_u64;
    let capabilities = candidate_command(
        context,
        policy,
        &rootfs,
        "capabilities",
        &["capabilities"],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let capability_records = compact("userland capabilities", &capabilities)?;
    let registry = required_record(&capability_records, "registry")?;
    let cli_contract = required_field(registry, "cli")?
        .parse::<u64>()
        .map_err(|_| DevError::corrupt("userland CLI contract is not an integer"))?;
    let registry_digest = required_field(registry, "digest")?.to_owned();
    let project = "/work/application";
    let created = candidate_command(
        context,
        policy,
        &rootfs,
        "new-command-project",
        &[
            "new",
            project,
            "--template",
            "command",
            "--name",
            "admission",
        ],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let created_records = compact("userland new", &created)?;
    require_field(&created_records, "project", "template", "command")?;
    let initial_revision =
        required_field(required_record(&created_records, "revision")?, "id")?.to_owned();
    candidate_command(
        context,
        policy,
        &rootfs,
        "status-initial",
        &["--project", project, "status"],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let owners = candidate_command(
        context,
        policy,
        &rootfs,
        "query-pure-function",
        &[
            "--project",
            project,
            "query",
            "owners",
            "--kind",
            "pure_function",
            "--limit",
            "20",
        ],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let owner_records = compact("userland query", &owners)?;
    let owner = required_record(&owner_records, "owner")?;
    require_exact(
        required_field(owner, "name")?,
        "greet",
        "command recipe function",
    )?;
    let owner_id = required_field(owner, "id")?.to_owned();
    let plan_path = "/work/rename.logical-plan";
    let planned = candidate_command(
        context,
        policy,
        &rootfs,
        "change-plan",
        &[
            "--project",
            project,
            "change",
            "plan",
            "rename.owner",
            "--base",
            &initial_revision,
            "--owner",
            &owner_id,
            "--name",
            "greet-admitted",
            "--output",
            plan_path,
        ],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let plan_records = compact("userland change plan", &planned)?;
    let plan_token = required_field(required_record(&plan_records, "plan")?, "token")?.to_owned();
    candidate_command(
        context,
        policy,
        &rootfs,
        "change-apply",
        &[
            "--project",
            project,
            "change",
            "apply",
            "rename.owner",
            "--base",
            &initial_revision,
            "--owner",
            &owner_id,
            "--name",
            "greet-admitted",
            "--plan",
            &plan_token,
        ],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let status = candidate_command(
        context,
        policy,
        &rootfs,
        "status-accepted",
        &["--project", project, "status"],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let status_records = compact("userland accepted status", &status)?;
    let accepted_revision =
        required_field(required_record(&status_records, "revision")?, "id")?.to_owned();
    if accepted_revision == initial_revision {
        return Err(DevError::corrupt(
            "userland reviewed change did not advance the accepted revision",
        ));
    }
    candidate_command(
        context,
        policy,
        &rootfs,
        "check",
        &["--project", project, "check"],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let artifact = "/work/application.lkja";
    candidate_command(
        context,
        policy,
        &rootfs,
        "build-incremental",
        &["--project", project, "build", "--output", artifact],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let artifact_host = rootfs.join("work/application.lkja");
    let (artifact_sha256, artifact_bytes) = archive::sha256_file(&artifact_host)?;
    let derived = rootfs.join("work/application/derived");
    let derived_metadata = fs::symlink_metadata(&derived)?;
    if derived_metadata.file_type().is_symlink() || !derived_metadata.is_dir() {
        return Err(DevError::corrupt(
            "userland derived state is not a regular directory",
        ));
    }
    fs::remove_dir_all(&derived)?;
    let clean_artifact = "/work/application-clean.lkja";
    candidate_command(
        context,
        policy,
        &rootfs,
        "build-clean",
        &["--project", project, "build", "--output", clean_artifact],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let (clean_sha256, clean_bytes) =
        archive::sha256_file(&rootfs.join("work/application-clean.lkja"))?;
    if artifact_bytes != clean_bytes || artifact_sha256 != clean_sha256 {
        return Err(DevError::corrupt(
            "userland clean and incremental artifacts differ",
        ));
    }
    let run = candidate_command(
        context,
        policy,
        &rootfs,
        "run-main",
        &["--project", project, "run", "main"],
        Expected::Passed,
    )?;
    candidate_commands += 1;
    let run_records = compact("userland run", &run)?;
    let execution = required_record(&run_records, "execution")?;
    let execution_value = required_field(execution, "value")?.to_owned();
    let differential_equal = required_field(execution, "differential")? == "equal";
    if !differential_equal {
        return Err(DevError::corrupt(
            "userland production and reference execution disagree",
        ));
    }
    let rejected = candidate_command(
        context,
        policy,
        &rootfs,
        "unknown-operation-rejected",
        &["unknown-operation"],
        Expected::Exit(2),
    )?;
    candidate_commands += 1;
    let rejected_records = compact("userland rejected command", &rejected)?;
    require_field(&rejected_records, "result", "status", "failure")?;

    let temporary_root_text = temporary_path.display().to_string();
    temporary.close().map_err(|error| {
        DevError::infrastructure(format!("remove userland temporary root: {error}"))
    })?;
    Ok(UserlandObservation {
        role: policy.role.clone(),
        status: AdmissionStatus::Passed,
        image,
        expected_libc: policy.expected_libc.clone(),
        os_release_sha256,
        os_release,
        rootfs_archive_bytes: rootfs_bytes,
        rootfs_archive_sha256: rootfs_sha256.as_str().to_owned(),
        candidate_sha256: copied.sha256,
        candidate_mode: copied.mode,
        network_policy: "sudo-unshare-network-namespace-no-host-library-mounts".to_owned(),
        candidate_commands,
        cli_contract,
        registry_digest,
        initial_revision,
        accepted_revision,
        artifact_bytes,
        artifact_sha256: artifact_sha256.as_str().to_owned(),
        clean_artifact_sha256: clean_sha256.as_str().to_owned(),
        execution_value,
        differential_equal,
        container_cleanup_complete: true,
        temporary_root_removed: !Path::new(&temporary_root_text).exists(),
    })
}

fn candidate_command(
    context: &mut Context,
    policy: &UserlandPolicy,
    rootfs: &Path,
    name: &str,
    arguments: &[&str],
    expected: Expected,
) -> Result<Vec<u8>, DevError> {
    let uid = rustix::process::getuid().as_raw();
    let gid = rustix::process::getgid().as_raw();
    let mut command = vec![
        "/usr/bin/sudo".to_owned(),
        "-n".to_owned(),
        "/usr/bin/env".to_owned(),
        "-i".to_owned(),
        "LANG=C".to_owned(),
        "PATH=/usr/sbin:/usr/bin:/sbin:/bin".to_owned(),
        "/usr/bin/unshare".to_owned(),
        "--net".to_owned(),
        "--".to_owned(),
        "/usr/sbin/chroot".to_owned(),
        format!("--userspec={uid}:{gid}"),
        rootfs.display().to_string(),
        "/work/lkjscript".to_owned(),
    ];
    command.extend(arguments.iter().map(|item| (*item).to_owned()));
    context.invoke(
        &format!("{}-candidate-{name}", policy.role),
        command,
        expected,
        COMMAND_TIMEOUT,
        false,
    )
}

fn run_oracles(
    context: &mut Context,
    repository: &Path,
    verifier: &Path,
    candidate: &Path,
) -> Result<Vec<OracleObservation>, DevError> {
    let distributed_root = context.evidence_root.join("distributed-http");
    let distributed_output = context.invoke(
        "distributed-http-oracle",
        vec![
            verifier.display().to_string(),
            "distributed-http".to_owned(),
            "--binary".to_owned(),
            candidate.display().to_string(),
            "--evidence-root".to_owned(),
            distributed_root.display().to_string(),
            "--machine".to_owned(),
        ],
        Expected::Passed,
        ORACLE_TIMEOUT,
        false,
    )?;
    require_machine_passed("distributed HTTP", &distributed_output)?;
    let distributed_receipt = distributed_root.join("receipt.json");
    let distributed =
        distributed_http::read_transferred_receipt(&distributed_receipt, candidate, verifier)?;

    let stateful_root = context.evidence_root.join("stateful-http");
    let stateful_output = context.invoke(
        "stateful-http-oracle",
        vec![
            verifier.display().to_string(),
            "stateful-http".to_owned(),
            "--binary".to_owned(),
            candidate.display().to_string(),
            "--evidence-root".to_owned(),
            stateful_root.display().to_string(),
            "--machine".to_owned(),
        ],
        Expected::Passed,
        ORACLE_TIMEOUT,
        false,
    )?;
    require_machine_passed("stateful HTTP", &stateful_output)?;
    let stateful_receipt = stateful_root.join("receipt.json");
    let stateful = stateful_http::read_transferred_receipt(&stateful_receipt, candidate, verifier)?;

    let service_output = context.invoke(
        "service-oracle",
        vec![
            verifier.display().to_string(),
            "service".to_owned(),
            "--binary".to_owned(),
            candidate.display().to_string(),
            "--machine".to_owned(),
        ],
        Expected::Passed,
        ORACLE_TIMEOUT,
        false,
    )?;
    let service_summary = require_machine_passed("service", &service_output)?;
    let service_receipt_text = service_summary
        .get("receipt")
        .and_then(Value::as_str)
        .ok_or_else(|| DevError::corrupt("service summary omitted receipt"))?;
    let service_receipt_relative = Path::new(service_receipt_text);
    if service_receipt_relative.is_absolute()
        || service_receipt_relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(DevError::corrupt(
            "service summary receipt path is not a canonical repository-relative path",
        ));
    }
    let service_receipt = repository.join(service_receipt_relative);
    let service = service::read_receipt(&service_receipt, candidate)?;
    let candidate_sha256 = target::observe_candidate(candidate)?.sha256;
    Ok(vec![
        OracleObservation {
            name: "distributed_http".to_owned(),
            status: AdmissionStatus::Passed,
            receipt: external_evidence(&distributed_receipt)?,
            verifier_sha256: distributed.verifier_sha256,
            candidate_sha256: distributed.candidate_sha256,
            elapsed_nanoseconds: distributed.elapsed_nanoseconds,
            commands: distributed.commands,
            runners: distributed.runners,
            requests: distributed.responses,
            cleanup_complete: distributed.cleanup_complete,
            prerequisite: "none".to_owned(),
        },
        OracleObservation {
            name: "stateful_http".to_owned(),
            status: AdmissionStatus::Passed,
            receipt: external_evidence(&stateful_receipt)?,
            verifier_sha256: stateful.verifier_sha256,
            candidate_sha256: stateful.candidate_sha256,
            elapsed_nanoseconds: stateful.elapsed_nanoseconds,
            commands: stateful.commands,
            runners: 3,
            requests: stateful.requests,
            cleanup_complete: stateful.cleanup_complete,
            prerequisite: stateful.postgres_identity,
        },
        OracleObservation {
            name: "service_acceptance".to_owned(),
            status: AdmissionStatus::Passed,
            receipt: external_evidence(&service_receipt)?,
            verifier_sha256: executable_binding(verifier)?.sha256,
            candidate_sha256,
            elapsed_nanoseconds: service.elapsed_nanoseconds,
            commands: service.commands,
            runners: service.runners,
            requests: service.requests,
            cleanup_complete: service.cleanup_complete,
            prerequisite: service::POSTGRES_IMAGE.to_owned(),
        },
    ])
}

impl Context {
    fn external_success(
        &mut self,
        name: &str,
        command: Vec<String>,
        timeout: Duration,
    ) -> Result<Vec<u8>, DevError> {
        self.invoke(name, command, Expected::Passed, timeout, true)
    }

    fn invoke(
        &mut self,
        name: &str,
        command: Vec<String>,
        expected: Expected,
        timeout: Duration,
        unavailable: bool,
    ) -> Result<Vec<u8>, DevError> {
        let ordinal = self.ordinal;
        self.ordinal = self
            .ordinal
            .checked_add(1)
            .ok_or_else(|| DevError::infrastructure("admission command ordinal overflow"))?;
        let safe = safe_name(name)?;
        let stdout = self
            .evidence_root
            .join(format!("{ordinal:04}-{safe}.stdout.log"));
        let stderr = self
            .evidence_root
            .join(format!("{ordinal:04}-{safe}.stderr.log"));
        let observation = process::run(
            &ProcessSpec {
                command: command.clone(),
                cwd: self.evidence_root.clone(),
                environment: process::environment(),
                timeout,
                maximum_stdout_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
                maximum_stderr_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
                stdout_path: stdout.clone(),
                stderr_path: stderr,
                unavailable_exit_code: None,
            },
            &self.evidence_root,
        );
        let expected_text = match expected {
            Expected::Passed => "exit-0".to_owned(),
            Expected::Exit(code) => format!("exit-{code}"),
        };
        let matched = match expected {
            Expected::Passed => observation.status == ProcessStatus::Passed,
            Expected::Exit(code) => observation.exit_code == Some(code),
        };
        let observed_status = observation.status;
        let observed_exit = observation.exit_code;
        let observed_reason = observation.reason.clone();
        self.commands.push(CommandEvidence {
            name: name.to_owned(),
            command,
            expected: expected_text,
            process: observation,
        });
        if !matched {
            let message = format!(
                "target admission command '{name}' ended as {observed_status:?} exit={observed_exit:?}: {}",
                observed_reason.as_deref().unwrap_or("no reason")
            );
            return Err(if unavailable {
                DevError::unavailable(message)
            } else {
                DevError::corrupt(message)
            });
        }
        process::read_bounded(&stdout, MAXIMUM_COMMAND_OUTPUT_BYTES)
    }
}

fn inspect_image(bytes: &[u8], policy: &UserlandPolicy) -> Result<ImageIdentity, DevError> {
    let values: Vec<Value> = serde_json::from_slice(bytes)
        .map_err(|error| DevError::corrupt(format!("decode Docker image inspection: {error}")))?;
    let value = values
        .first()
        .filter(|_| values.len() == 1)
        .ok_or_else(|| DevError::corrupt("Docker image inspection did not contain one object"))?;
    let image_id = required_json_text(value, "Id")?.to_owned();
    let operating_system = required_json_text(value, "Os")?.to_owned();
    let architecture = required_json_text(value, "Architecture")?.to_owned();
    let virtual_size_bytes = value
        .get("Size")
        .and_then(Value::as_u64)
        .ok_or_else(|| DevError::corrupt("Docker image inspection omitted Size"))?;
    let mut repository_digests = value
        .get("RepoDigests")
        .and_then(Value::as_array)
        .ok_or_else(|| DevError::corrupt("Docker image inspection omitted RepoDigests"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| DevError::corrupt("Docker RepoDigests entry is not text"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    repository_digests.sort();
    repository_digests.dedup();
    if !image_id.starts_with("sha256:")
        || !repository_digests.iter().any(|item| item == &policy.image)
        || operating_system != policy.operating_system
        || architecture != policy.architecture
        || virtual_size_bytes == 0
        || virtual_size_bytes > MAXIMUM_ROOTFS_ARCHIVE_BYTES
    {
        return Err(DevError::corrupt(
            "Docker image inspection disagrees with the exact userland policy",
        ));
    }
    Ok(ImageIdentity {
        requested: policy.image.clone(),
        image_id,
        repository_digests,
        operating_system,
        architecture,
        virtual_size_bytes,
    })
}

fn rootfs_os_release(
    rootfs: &Path,
    policy: &UserlandPolicy,
) -> Result<(String, Vec<String>), DevError> {
    let requested = rootfs.join("etc/os-release");
    let metadata = fs::symlink_metadata(&requested)?;
    let path = if metadata.file_type().is_symlink() {
        let target = fs::read_link(&requested)?;
        if target != Path::new("/usr/lib/os-release")
            && target != Path::new("../usr/lib/os-release")
        {
            return Err(DevError::corrupt(
                "userland /etc/os-release has an unexpected symlink target",
            ));
        }
        rootfs.join("usr/lib/os-release")
    } else {
        requested
    };
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 64 * 1024 {
        return Err(DevError::corrupt(
            "userland os-release is unsafe or oversized",
        ));
    }
    let bytes = fs::read(&path)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| DevError::corrupt("userland os-release is not UTF-8"))?;
    let lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
    let expected_id = if policy.role == "musl" {
        "ID=alpine"
    } else {
        "ID=debian"
    };
    if !lines.iter().any(|line| line == expected_id) {
        return Err(DevError::corrupt(
            "userland os-release disagrees with its maintained role",
        ));
    }
    let digest = archive::sha256_bytes(&bytes)?;
    Ok((digest.as_str().to_owned(), lines))
}

fn require_machine_passed(label: &str, bytes: &[u8]) -> Result<Value, DevError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| DevError::corrupt(format!("{label} summary is not UTF-8")))?;
    let mut lines = text.lines();
    let line = lines
        .next()
        .ok_or_else(|| DevError::corrupt(format!("{label} summary is empty")))?;
    if lines.next().is_some() {
        return Err(DevError::corrupt(format!(
            "{label} summary contains more than one line"
        )));
    }
    let value: Value = serde_json::from_str(line)
        .map_err(|error| DevError::corrupt(format!("decode {label} summary: {error}")))?;
    if value.get("status").and_then(Value::as_str) != Some("passed") {
        return Err(DevError::corrupt(format!(
            "{label} did not report a passed classification"
        )));
    }
    Ok(value)
}

fn compact(label: &str, bytes: &[u8]) -> Result<Vec<CompactRecord>, DevError> {
    parse_records(label, bytes).map_err(|diagnostics| {
        DevError::corrupt(format!(
            "{label} did not emit canonical compact records: {}",
            diagnostics
                .first()
                .map(|item| item.message.as_str())
                .unwrap_or("unknown compact-record error")
        ))
    })
}

fn required_record<'a>(
    records: &'a [CompactRecord],
    operation: &str,
) -> Result<&'a CompactRecord, DevError> {
    records
        .iter()
        .find(|record| record.operation == operation)
        .ok_or_else(|| DevError::corrupt(format!("missing '{operation}' compact record")))
}

fn required_field<'a>(record: &'a CompactRecord, name: &str) -> Result<&'a str, DevError> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
        .ok_or_else(|| {
            DevError::corrupt(format!(
                "compact record '{}' omitted field '{name}'",
                record.operation
            ))
        })
}

fn require_field(
    records: &[CompactRecord],
    operation: &str,
    field: &str,
    expected: &str,
) -> Result<(), DevError> {
    require_exact(
        required_field(required_record(records, operation)?, field)?,
        expected,
        field,
    )
}

fn require_exact(observed: &str, expected: &str, label: &str) -> Result<(), DevError> {
    if observed == expected {
        Ok(())
    } else {
        Err(DevError::corrupt(format!(
            "{label} was '{observed}', expected '{expected}'"
        )))
    }
}

fn executable_binding(path: &Path) -> Result<ExecutableBinding, DevError> {
    let path = path.canonicalize()?;
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.permissions().mode() & 0o111 == 0
    {
        return Err(DevError::usage(
            "target admission verifier is not a regular executable",
        ));
    }
    let (sha256, byte_length) = archive::sha256_file(&path)?;
    Ok(ExecutableBinding {
        path: path.display().to_string(),
        byte_length,
        mode: metadata.permissions().mode() & 0o7777,
        sha256: sha256.as_str().to_owned(),
    })
}

fn current_verifier() -> Result<PathBuf, DevError> {
    std::env::current_exe()
        .map_err(|error| DevError::infrastructure(format!("resolve admission verifier: {error}")))?
        .canonicalize()
        .map_err(DevError::from)
}

fn external_evidence(path: &Path) -> Result<ExternalEvidence, DevError> {
    let path = path.canonicalize()?;
    let (sha256, byte_length) = archive::sha256_file(&path)?;
    Ok(ExternalEvidence {
        path: path.display().to_string(),
        byte_length,
        sha256,
    })
}

fn host_tools(repository: &Path) -> Result<HostTools, DevError> {
    Ok(HostTools {
        docker: super::command_text("docker", &["--version"], repository, 64 * 1024)?,
        sudo: super::command_text("/usr/bin/sudo", &["--version"], repository, 64 * 1024)?
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned(),
        unshare: super::command_text("/usr/bin/unshare", &["--version"], repository, 64 * 1024)?,
        chroot: super::command_text("/usr/sbin/chroot", &["--version"], repository, 64 * 1024)?,
        tar: super::command_text("tar", &["--version"], repository, 64 * 1024)?
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned(),
    })
}

fn required_json_text<'a>(value: &'a Value, name: &str) -> Result<&'a str, DevError> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| DevError::corrupt(format!("Docker image inspection omitted {name}")))
}

fn fresh(name: &str, detail: &str) -> VerificationClassification {
    VerificationClassification {
        name: name.to_owned(),
        classification: EvidenceClassification::FreshPassed,
        detail: detail.to_owned(),
    }
}

fn parse_options(mut arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let mut values = BTreeMap::new();
    while let Some(argument) = crate::next_utf8(&mut arguments, "target admission option")? {
        if !matches!(
            argument.as_str(),
            "--candidate" | "--build-receipt" | "--evidence-root"
        ) {
            return Err(DevError::usage(format!(
                "unknown target admission option '{argument}'"
            )));
        }
        let value = crate::next_utf8(&mut arguments, "target admission option value")?
            .ok_or_else(|| DevError::usage(format!("{argument} requires a path")))?;
        if values
            .insert(argument.clone(), PathBuf::from(value))
            .is_some()
        {
            return Err(DevError::usage(format!(
                "duplicate target admission option '{argument}'"
            )));
        }
    }
    let take = |name: &str| {
        values
            .get(name)
            .cloned()
            .ok_or_else(|| DevError::usage(format!("{name} is required")))
    };
    Ok(Options {
        candidate: take("--candidate")?,
        build_receipt: take("--build-receipt")?,
        evidence_root: take("--evidence-root")?,
    })
}

fn require_absent_root(path: &Path) -> Result<(), DevError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|item| matches!(item, Component::CurDir | Component::ParentDir))
    {
        return Err(DevError::usage(
            "target admission evidence root must be absolute and lexically canonical",
        ));
    }
    if fs::symlink_metadata(path).is_ok() {
        return Err(DevError::usage(
            "target admission evidence root must not already exist",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| DevError::usage("target admission evidence root has no parent"))?;
    let metadata = fs::symlink_metadata(parent)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || parent.canonicalize()? != parent {
        return Err(DevError::usage(
            "target admission evidence-root parent is not a canonical real directory",
        ));
    }
    Ok(())
}

fn create_evidence_root(path: &Path) -> Result<PathBuf, DevError> {
    require_absent_root(path)?;
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    fs::File::open(
        path.parent()
            .ok_or_else(|| DevError::infrastructure("evidence root has no parent"))?,
    )?
    .sync_all()?;
    path.canonicalize().map_err(DevError::from)
}

fn safe_name(value: &str) -> Result<String, DevError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(DevError::usage("admission evidence name is not portable"));
    }
    Ok(value.to_owned())
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_admission_options_are_closed_and_complete() {
        let parse = |values: &[&str]| parse_options(values.iter().map(OsString::from));
        assert!(parse(&[]).is_err());
        assert!(parse(&["--candidate", "/a"]).is_err());
        assert!(parse(&["--unknown", "/a"]).is_err());
        assert!(
            parse(&[
                "--candidate",
                "/a",
                "--candidate",
                "/b",
                "--build-receipt",
                "/c",
                "--evidence-root",
                "/d",
            ])
            .is_err()
        );
        let parsed = parse(&[
            "--candidate",
            "/a",
            "--build-receipt",
            "/b",
            "--evidence-root",
            "/c",
        ])
        .expect("complete admission options");
        assert_eq!(parsed.candidate, Path::new("/a"));
    }

    #[test]
    fn admission_schema_and_required_classifications_are_stable() {
        assert_eq!(ADMISSION_SCHEMA, "lkjscript-target-admission-receipt");
        assert_eq!(ADMISSION_SCHEMA_VERSION, 1);
        assert_eq!(
            [
                "static_linkage",
                "musl_userland",
                "older_glibc_userland",
                "distributed_http",
                "stateful_http",
                "service_acceptance",
            ]
            .len(),
            6
        );
    }

    #[test]
    fn evidence_root_is_create_new_absolute_and_non_symlinked() {
        let temporary = tempfile::tempdir().expect("temporary admission parent");
        assert!(require_absent_root(Path::new("relative")).is_err());
        let root = temporary.path().join("admission");
        let created = create_evidence_root(&root).expect("create admission root");
        assert_eq!(created, root);
        assert_eq!(
            fs::metadata(&created)
                .expect("admission root metadata")
                .permissions()
                .mode()
                & 0o7777,
            0o700
        );
        assert!(require_absent_root(&created).is_err());
    }
}
