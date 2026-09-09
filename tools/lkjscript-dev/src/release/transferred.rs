//! Finite transferred acceptance. Behavioral meaning and receipt validation stay with each owner.
use super::{archive, model::*, target, verifier};
use crate::{
    distributed_http, error::DevError, evidence, offline_packages, outbound_http, process,
    pure_tail, stateful_http,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SCHEMA: &str = "lkjscript-transferred-acceptance";
const SCHEMA_VERSION: u32 = 1;
const MAXIMUM_RECEIPT_BYTES: u64 = 128 * 1024 * 1024;
const MAXIMUM_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Oracle {
    DistributedHttp,
    OutboundHttp,
    OfflinePackages,
    PureTail,
    StatefulHttp,
}

// The only production inventory, also used by target admission and the verifier handoff.
pub(super) const ORACLES: [Oracle; 5] = [
    Oracle::DistributedHttp,
    Oracle::OutboundHttp,
    Oracle::OfflinePackages,
    Oracle::PureTail,
    Oracle::StatefulHttp,
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChildFacts {
    pub(super) candidate_sha256: String,
    pub(super) verifier_sha256: String,
    pub(super) elapsed_nanoseconds: u64,
    pub(super) commands: u64,
    pub(super) runners: u64,
    pub(super) requests: u64,
    pub(super) cleanup_complete: bool,
}

impl Oracle {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::DistributedHttp => "distributed-http",
            Self::OutboundHttp => "outbound-http",
            Self::OfflinePackages => "offline-packages",
            Self::PureTail => "pure-tail",
            Self::StatefulHttp => "stateful-http",
        }
    }
    pub(super) fn admission_name(self) -> &'static str {
        match self {
            Self::DistributedHttp => "distributed_http",
            Self::OutboundHttp => "outbound_http",
            Self::OfflinePackages => "offline_packages",
            Self::PureTail => "pure_tail",
            Self::StatefulHttp => "stateful_http",
        }
    }
    pub(super) fn prerequisite(self) -> &'static str {
        match self {
            Self::DistributedHttp => "none",
            Self::OutboundHttp => "http_client_adapter_1",
            Self::OfflinePackages => "code_complete_package_container_1",
            Self::PureTail => "pure-tail-execution",
            Self::StatefulHttp => "lkjscript-data-store-1",
        }
    }
    pub(super) fn timeout(self) -> Duration {
        Duration::from_secs(if self == Self::PureTail { 900 } else { 1200 })
    }
    pub(super) fn read(
        self,
        receipt: &Path,
        candidate: &Path,
        verifier: &Path,
    ) -> Result<ChildFacts, DevError> {
        regular(receipt)?;
        Ok(match self {
            Self::DistributedHttp => {
                let r = distributed_http::read_transferred_receipt(receipt, candidate, verifier)?;
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands: r.commands,
                    runners: r.runners,
                    requests: r.responses,
                    cleanup_complete: r.cleanup_complete,
                }
            }
            Self::OutboundHttp => {
                let r = outbound_http::read_transferred_receipt(receipt, candidate, verifier)?;
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands: r.commands,
                    runners: r.runners,
                    requests: r.requests,
                    cleanup_complete: r.cleanup_complete,
                }
            }
            Self::OfflinePackages => {
                let r = offline_packages::read_transferred_receipt(receipt, candidate, verifier)?;
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands: u64::try_from(r.commands.len())
                        .map_err(|_| DevError::corrupt("command count overflow"))?,
                    runners: u64::try_from(r.runners.len())
                        .map_err(|_| DevError::corrupt("runner count overflow"))?,
                    requests: 1,
                    cleanup_complete: r.cleanup_complete,
                }
            }
            Self::PureTail => {
                let r = pure_tail::read_transferred_receipt(receipt, candidate, verifier)?;
                let commands = r.command_count();
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands,
                    runners: 1,
                    requests: 4,
                    cleanup_complete: r.cleanup_complete,
                }
            }
            Self::StatefulHttp => {
                let r = stateful_http::read_transferred_receipt(receipt, candidate, verifier)?;
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands: r.commands,
                    runners: 3,
                    requests: r.requests,
                    cleanup_complete: r.cleanup_complete,
                }
            }
        })
    }
}

pub(super) fn roles() -> Vec<&'static str> {
    std::iter::once("release-verify")
        .chain(ORACLES.map(Oracle::name))
        .collect()
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Boundary {
    PrePublication,
    ExactDownload,
    LatestDownload,
}

#[derive(Debug)]
struct Options {
    verify: bool,
    candidate: PathBuf,
    manifest: PathBuf,
    tag: String,
    commit: String,
    publication: PublicationMode,
    boundary: Boundary,
    verifier_identity: PathBuf,
    expected_verifier_sha256: String,
    expected_verifier_bytes: u64,
    evidence_root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ReceiptIdentity {
    file: ExternalEvidence,
    digest: evidence::VerificationDigest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    FreshPassed,
    Failed,
    Unavailable,
    NotRun,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Child {
    role: Oracle,
    evidence_root: String,
    status: Status,
    command: Vec<String>,
    process: Option<process::ProcessObservation>,
    receipt: Option<ReceiptIdentity>,
    facts: Option<ChildFacts>,
    cleanup_complete: bool,
    failure: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: SchemaIdentity,
    status: Status,
    boundary: Boundary,
    tag: String,
    source_commit: String,
    publication: PublicationMode,
    product: ProductIdentity,
    capabilities_digest: String,
    target: String,
    target_policy_sha256: String,
    evidence_root: String,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: Option<u128>,
    elapsed_nanoseconds: u64,
    hosted_context: HostedContext,
    manifest: ExternalEvidence,
    candidate: target::BuiltCandidate,
    verifier: ExternalEvidence,
    verifier_mode: u32,
    children: Vec<Child>,
    cleanup_complete: bool,
    failure: Option<String>,
}

pub(super) fn command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let operation = crate::next_utf8(&mut arguments, "transferred operation")?
        .ok_or_else(|| DevError::usage("release transferred requires run or verify"))?;
    let options = parse_options(&operation, arguments)?;
    let verifier_path = std::env::current_exe()?.canonicalize()?;
    verifier::validate_handoff(
        &verifier_path,
        &options.verifier_identity,
        &options.tag,
        &options.commit,
        &options.expected_verifier_sha256,
        options.expected_verifier_bytes,
    )?;
    let manifest = load_manifest(&options)?;
    if options.verify {
        let receipt = read_receipt(&options, &verifier_path, &manifest)?;
        print_summary(&options.evidence_root.join("receipt.json"), &receipt)?;
        return Ok(0);
    }
    super::require_absolute_extraction_output(&options.evidence_root)?;
    fs::create_dir(&options.evidence_root)?;
    fs::set_permissions(&options.evidence_root, fs::Permissions::from_mode(0o700))?;
    let started = Instant::now();
    let mut receipt = Receipt {
        schema: SchemaIdentity {
            identity: SCHEMA.to_owned(),
            version: SCHEMA_VERSION,
        },
        status: Status::NotRun,
        boundary: options.boundary,
        tag: options.tag.clone(),
        source_commit: options.commit.clone(),
        publication: options.publication,
        product: manifest.product.clone(),
        capabilities_digest: manifest.executable.capabilities_digest.clone(),
        target: manifest.target_triple.clone(),
        target_policy_sha256: target::policy_sha256()?,
        evidence_root: options.evidence_root.display().to_string(),
        started_unix_nanoseconds: super::unix_nanoseconds()?,
        completed_unix_nanoseconds: None,
        elapsed_nanoseconds: 0,
        hosted_context: super::hosted_context(),
        manifest: external(&options.manifest)?,
        candidate: target::observe_candidate(&options.candidate)?,
        verifier: external(&verifier_path)?,
        verifier_mode: 0o755,
        children: ORACLES
            .into_iter()
            .map(|role| Child {
                role,
                evidence_root: options
                    .evidence_root
                    .join(role.name())
                    .display()
                    .to_string(),
                status: Status::NotRun,
                command: vec![
                    verifier_path.display().to_string(),
                    role.name().to_owned(),
                    "--binary".to_owned(),
                    options.candidate.display().to_string(),
                    "--evidence-root".to_owned(),
                    options
                        .evidence_root
                        .join(role.name())
                        .display()
                        .to_string(),
                    "--machine".to_owned(),
                ],
                process: None,
                receipt: None,
                facts: None,
                cleanup_complete: false,
                failure: None,
            })
            .collect(),
        cleanup_complete: false,
        failure: None,
    };
    let path = options.evidence_root.join("receipt.json");
    // Persist incomplete state before any candidate/child execution. An interrupted attempt never looks passed.
    evidence::publish_json(&path, &receipt)?;
    let result = Cancellation::new().and_then(|cancellation| {
        run_children(
            &options,
            &verifier_path,
            &manifest,
            &mut receipt,
            &path,
            &cancellation.control,
        )
    });
    receipt.completed_unix_nanoseconds = Some(super::unix_nanoseconds()?);
    receipt.elapsed_nanoseconds = u64::try_from(started.elapsed().as_nanos())
        .map_err(|_| DevError::corrupt("transfer elapsed overflow"))?;
    receipt.cleanup_complete = receipt.children.iter().all(|child| child.cleanup_complete);
    match result {
        Ok(()) => receipt.status = Status::FreshPassed,
        Err(error) => {
            receipt.status = Status::Failed;
            receipt.failure = Some(error.to_string());
        }
    }
    if receipt.status == Status::FreshPassed
        && let Err(error) = validate_receipt(&receipt, &options, &verifier_path, &manifest)
    {
        receipt.status = Status::Failed;
        receipt.failure = Some(error.to_string());
    }
    evidence::publish_json(&path, &receipt)?;
    print_summary(&path, &receipt)?;
    Ok(if receipt.status == Status::FreshPassed {
        0
    } else {
        1
    })
}

fn run_children(
    options: &Options,
    verifier: &Path,
    manifest: &ReleaseManifest,
    receipt: &mut Receipt,
    path: &Path,
    control: &process::ProcessControl,
) -> Result<(), DevError> {
    let capabilities = super::inspect_capabilities(&options.candidate, &options.evidence_root)?;
    require(
        capabilities.product_version == manifest.product.version
            && capabilities.capabilities_digest == manifest.executable.capabilities_digest,
        "actual candidate capabilities differ from manifest",
    )?;
    for index in 0..receipt.children.len() {
        require(!control.cancelled(), "transferred acceptance cancelled")?;
        check_inputs(options, verifier, receipt)?;
        // Child temporary projects/services live in an owned root; receipt/log roots remain separate.
        let scratch = tempfile::Builder::new()
            .prefix(".child-state-")
            .tempdir_in(&options.evidence_root)?;
        let child = &mut receipt.children[index];
        child.status = Status::Failed;
        child.failure = Some("child invocation incomplete".to_owned());
        evidence::publish_json(path, receipt)?;
        let child = &mut receipt.children[index];
        let spec = process::ProcessSpec {
            command: child.command.clone(),
            cwd: scratch.path().to_path_buf(),
            environment: BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
                ("TMPDIR".to_owned(), scratch.path().display().to_string()),
            ]),
            timeout: child.role.timeout(),
            maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
            stdout_path: options
                .evidence_root
                .join(format!("{}.stdout.log", child.role.name())),
            stderr_path: options
                .evidence_root
                .join(format!("{}.stderr.log", child.role.name())),
            unavailable_exit_code: Some(2),
        };
        let observation = process::run_supervised(&spec, &options.evidence_root, Some(control));
        let successful = observation.status == process::ProcessStatus::Passed;
        if observation.status == process::ProcessStatus::Unavailable {
            child.status = Status::Unavailable;
        }
        child.process = Some(observation);
        let child_path = Path::new(&child.evidence_root).join("receipt.json");
        if child_path.try_exists()? {
            child.receipt = Some(receipt_identity(&child_path)?);
        }
        let result = child.role.read(&child_path, &options.candidate, verifier);
        let scratch_closed = scratch.close();
        match result {
            Ok(facts) if successful && scratch_closed.is_ok() => {
                child.cleanup_complete = facts.cleanup_complete;
                child.facts = Some(facts);
                child.status = Status::FreshPassed;
                child.failure = None;
            }
            result => {
                let reason = match result {
                    Err(error) => error.to_string(),
                    Ok(_) => "child process failed or temporary cleanup failed".to_owned(),
                };
                child.failure = Some(reason.clone());
                return Err(DevError::corrupt(reason));
            }
        }
        check_inputs(options, verifier, receipt)?;
        evidence::publish_json(path, receipt)?;
    }
    Ok(())
}

// The command owns this signal lifetime and joins it before returning. No signal handler runs
// application work; it only asks the bounded child supervisor to terminate its owned tree.
struct Cancellation {
    control: process::ProcessControl,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Cancellation {
    fn new() -> Result<Self, DevError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()?;
        let (mut interrupt, mut terminate) = {
            let _entered = runtime.enter();
            (
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?,
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?,
            )
        };
        let control = process::ProcessControl::default();
        let signal_control = control.clone();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("transfer-cancellation".to_owned())
            .spawn(move || {
                runtime.block_on(async move {
                    tokio::select! {
                        _ = interrupt.recv() => signal_control.kill(),
                        _ = terminate.recv() => signal_control.kill(),
                        _ = stopped => (),
                    }
                });
            })?;
        Ok(Self {
            control,
            stop: Some(stop),
            thread: Some(thread),
        })
    }
}
impl Drop for Cancellation {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn load_manifest(options: &Options) -> Result<ReleaseManifest, DevError> {
    regular(&options.candidate)?;
    regular(&options.manifest)?;
    let bytes = process::read_bounded(&options.manifest, 1024 * 1024)?;
    let manifest: ReleaseManifest = serde_json::from_slice(&bytes)?;
    require(
        archive::canonical_json(&manifest)? == bytes,
        "transferred manifest is noncanonical",
    )?;
    super::validate_manifest(&manifest)?;
    require(
        options.candidate.file_name().and_then(|v| v.to_str()) == Some("lkjscript")
            && options.manifest.file_name().and_then(|v| v.to_str())
                == Some("RELEASE-MANIFEST.json")
            && options.candidate.parent() == options.manifest.parent(),
        "candidate and manifest are not the exact extraction",
    )?;
    require(
        manifest.source.expected_release_tag == options.tag
            && manifest.source.tagged_commit_sha == options.commit
            && manifest.publication_mode == options.publication
            && manifest.target_triple == target::TARGET_TRIPLE
            && (options.boundary == Boundary::PrePublication
                || options.publication == PublicationMode::Release),
        "transferred release identity mismatch",
    )?;
    let candidate = target::observe_candidate(&options.candidate)?;
    require(
        candidate.sha256 == manifest.executable.sha256.as_str()
            && candidate.byte_length == manifest.executable.byte_length
            && candidate.mode == manifest.executable.archive_mode
            && candidate.elf == manifest.executable.elf,
        "transferred candidate differs from manifest",
    )?;
    Ok(manifest)
}

fn check_inputs(options: &Options, verifier: &Path, receipt: &Receipt) -> Result<(), DevError> {
    require(
        target::observe_candidate(&options.candidate)? == receipt.candidate
            && external(verifier)? == receipt.verifier
            && fs::metadata(verifier)?.permissions().mode() & 0o7777 == receipt.verifier_mode
            && external(&options.manifest)? == receipt.manifest,
        "candidate, verifier, or manifest changed during transferred acceptance",
    )
}

fn read_receipt(
    options: &Options,
    verifier: &Path,
    manifest: &ReleaseManifest,
) -> Result<Receipt, DevError> {
    let path = options.evidence_root.join("receipt.json");
    regular(&path)?;
    let bytes = process::read_bounded(&path, MAXIMUM_RECEIPT_BYTES)?;
    let receipt: Receipt = serde_json::from_slice(&bytes)?;
    require(
        evidence::encode_json(&receipt)? == bytes,
        "transferred receipt is noncanonical",
    )?;
    validate_receipt(&receipt, options, verifier, manifest)?;
    Ok(receipt)
}

fn validate_receipt(
    receipt: &Receipt,
    options: &Options,
    verifier: &Path,
    manifest: &ReleaseManifest,
) -> Result<(), DevError> {
    validate_inventory(&receipt.children)?;
    require(
        receipt.schema.identity == SCHEMA
            && receipt.schema.version == SCHEMA_VERSION
            && receipt.status == Status::FreshPassed
            && receipt.failure.is_none()
            && receipt.cleanup_complete
            && receipt.boundary == options.boundary
            && receipt.tag == options.tag
            && receipt.source_commit == options.commit
            && receipt.publication == options.publication
            && receipt.product == manifest.product
            && receipt.capabilities_digest == manifest.executable.capabilities_digest
            && receipt.target == target::TARGET_TRIPLE
            && receipt.target_policy_sha256 == target::policy_sha256()?
            && receipt.evidence_root == options.evidence_root.display().to_string()
            && receipt
                .completed_unix_nanoseconds
                .is_some_and(|time| time >= receipt.started_unix_nanoseconds),
        "transferred receipt is foreign, stale, or incomplete",
    )?;
    check_inputs(options, verifier, receipt)?;
    for child in &receipt.children {
        let root = options.evidence_root.join(child.role.name());
        require(
            child.evidence_root == root.display().to_string(),
            "child evidence root is foreign",
        )?;
        let path = root.join("receipt.json");
        require(
            child.receipt.as_ref() == Some(&receipt_identity(&path)?),
            "child evidence identity changed",
        )?;
        let expected = child.role.read(&path, &options.candidate, verifier)?;
        require(
            child.facts.as_ref() == Some(&expected),
            "child facts differ from owned receipt validation",
        )?;
        let expected_command = vec![
            verifier.display().to_string(),
            child.role.name().to_owned(),
            "--binary".to_owned(),
            options.candidate.display().to_string(),
            "--evidence-root".to_owned(),
            root.display().to_string(),
            "--machine".to_owned(),
        ];
        require(
            child.command == expected_command,
            "child invocation is foreign",
        )?;
        let observation = child
            .process
            .as_ref()
            .ok_or_else(|| DevError::corrupt("child process evidence missing"))?;
        for (proof, suffix) in [
            (&observation.stdout, "stdout.log"),
            (&observation.stderr, "stderr.log"),
        ] {
            let name = format!("{}.{suffix}", child.role.name());
            require(
                proof.path == name
                    && *proof == evidence::proof(&options.evidence_root.join(&name), name)?,
                "child command evidence changed",
            )?;
        }
    }
    Ok(())
}

fn validate_inventory(children: &[Child]) -> Result<(), DevError> {
    require(
        children.iter().map(|child| child.role).eq(ORACLES),
        "transferred inventory is missing, duplicated, extra, or reordered",
    )?;
    for child in children {
        require(
            child.status == Status::FreshPassed
                && child.cleanup_complete
                && child.failure.is_none()
                && child.receipt.is_some()
                && child
                    .facts
                    .as_ref()
                    .is_some_and(|facts| facts.cleanup_complete && facts.commands > 0)
                && child.process.as_ref().is_some_and(|p| {
                    p.status == process::ProcessStatus::Passed
                        && p.exit_code == Some(0)
                        && p.signal.is_none()
                        && !p.stdout_limit_exhausted
                        && !p.stderr_limit_exhausted
                }),
            "required child is failed, unavailable, unrun, or unclean",
        )?;
    }
    Ok(())
}

fn external(path: &Path) -> Result<ExternalEvidence, DevError> {
    regular(path)?;
    require(
        fs::metadata(path)?.len() <= 384 * 1024 * 1024,
        "transferred input exceeds byte bound",
    )?;
    let (sha256, byte_length) = archive::sha256_file(path)?;
    Ok(ExternalEvidence {
        path: path.display().to_string(),
        byte_length,
        sha256,
    })
}
fn receipt_identity(path: &Path) -> Result<ReceiptIdentity, DevError> {
    let file = external(path)?;
    let bytes = process::read_bounded(path, MAXIMUM_RECEIPT_BYTES)?;
    Ok(ReceiptIdentity {
        file,
        digest: evidence::VerificationDigest::of(&bytes),
    })
}
fn regular(path: &Path) -> Result<(), DevError> {
    super::require_absolute_regular(path, "transferred input")?;
    require(
        path.canonicalize()? == path,
        "transferred path contains a symlink or noncanonical component",
    )
}
fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}
fn print_summary(path: &Path, receipt: &Receipt) -> Result<(), DevError> {
    let identity = receipt_identity(path)?;
    println!(
        "{}",
        serde_json::json!({"status":if receipt.status == Status::FreshPassed {"passed"}else{"failed"},"boundary":receipt.boundary,
        "tag":receipt.tag,"source_commit":receipt.source_commit,"candidate_sha256":receipt.candidate.sha256,"verifier_sha256":receipt.verifier.sha256,
        "receipt":path,"receipt_bytes":identity.file.byte_length,"receipt_sha256":identity.file.sha256,"receipt_digest":identity.digest,
        "children":receipt.children.iter().map(|child|serde_json::json!({"role":child.role,"status":child.status,"receipt":child.receipt,"cleanup_complete":child.cleanup_complete})).collect::<Vec<_>>(),
        "cleanup_complete":receipt.cleanup_complete,"failure":receipt.failure})
    );
    Ok(())
}
fn parse_options(
    operation: &str,
    arguments: impl Iterator<Item = OsString>,
) -> Result<Options, DevError> {
    let verify = match operation {
        "run" => false,
        "verify" => true,
        _ => {
            return Err(DevError::usage(
                "transferred operation must be run or verify",
            ));
        }
    };
    let mut values = verifier::parse_values(
        arguments,
        &[
            "--candidate",
            "--manifest",
            "--tag",
            "--commit",
            "--publication",
            "--boundary",
            "--verifier-identity",
            "--expected-verifier-sha256",
            "--expected-verifier-bytes",
            "--evidence-root",
        ],
    )?;
    let tag = verifier::required(&mut values, "--tag")?;
    verifier::validate_tag(&tag)?;
    let commit = verifier::required(&mut values, "--commit")?;
    super::validate_git_sha(&commit, "transferred source")?;
    let publication = match verifier::required(&mut values, "--publication")?.as_str() {
        "dry-run" => PublicationMode::DryRun,
        "release" => PublicationMode::Release,
        _ => return Err(DevError::usage("publication must be dry-run or release")),
    };
    let boundary = match verifier::required(&mut values, "--boundary")?.as_str() {
        "pre-publication" => Boundary::PrePublication,
        "exact-download" => Boundary::ExactDownload,
        "latest-download" => Boundary::LatestDownload,
        _ => return Err(DevError::usage("invalid transfer boundary")),
    };
    let expected_verifier_sha256 = verifier::required(&mut values, "--expected-verifier-sha256")?;
    Sha256Digest::new(expected_verifier_sha256.clone()).map_err(DevError::usage)?;
    let expected_verifier_bytes = verifier::required(&mut values, "--expected-verifier-bytes")?
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| DevError::usage("expected verifier bytes must be positive"))?;
    Ok(Options {
        verify,
        candidate: PathBuf::from(verifier::required(&mut values, "--candidate")?),
        manifest: PathBuf::from(verifier::required(&mut values, "--manifest")?),
        tag,
        commit,
        publication,
        boundary,
        verifier_identity: PathBuf::from(verifier::required(&mut values, "--verifier-identity")?),
        expected_verifier_sha256,
        expected_verifier_bytes,
        evidence_root: PathBuf::from(verifier::required(&mut values, "--evidence-root")?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transferred_inventory_is_independently_specified() {
        assert_eq!(
            roles(),
            [
                "release-verify",
                "distributed-http",
                "outbound-http",
                "offline-packages",
                "pure-tail",
                "stateful-http"
            ]
        );
        assert_eq!(
            ORACLES.map(Oracle::admission_name),
            [
                "distributed_http",
                "outbound_http",
                "offline_packages",
                "pure_tail",
                "stateful_http"
            ]
        );
    }
    #[test]
    fn options_and_paths_reject_foreign_or_ambiguous_input() {
        assert!(parse_options("run", [].into_iter()).is_err());
        assert!(parse_options("fallback", [].into_iter()).is_err());
        assert!(parse_options("run", ["--unknown", "x"].into_iter().map(OsString::from)).is_err());
        assert!(regular(Path::new("relative")).is_err());
        let root = tempfile::tempdir().expect("root");
        let real = root.path().join("real");
        fs::write(&real, b"file").expect("file");
        std::os::unix::fs::symlink(&real, root.path().join("link")).expect("link");
        assert!(regular(&root.path().join("link")).is_err());
    }
}

#[cfg(test)]
#[path = "transferred/tests.rs"]
mod fault_tests;
