//! Copied-candidate acceptance for deployment-bound outbound HTTP and NIP-11 transport.
//!
//! The relay side deliberately uses a raw HTTP/1.1 implementation that does not depend on the
//! product client parser, endpoint policy, or application assertions. Rustls is shared only for
//! the cryptographic TLS record and certificate boundary.

use crate::authority::{self, AuthorityObservation};
use crate::error::DevError;
use crate::evidence::{self, FileProof, PublishedEvidence, VerificationDigest};
use crate::http_probe;
use crate::process::{self, ProcessControl, ProcessObservation, ProcessSpec, ProcessStatus};
use lkjscript::platform::control::{CompactRecord, parse_records};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose, PKCS_ED25519, SerialNumber, date_time_ymd,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const ACCEPTANCE_SCHEMA: &str = "lkjscript-outbound-http-acceptance";
const ACCEPTANCE_SCHEMA_VERSION: u32 = 1;
const ACCEPTANCE_WORKFLOW: &str = "outbound-http-application";
const FIXTURE_GENERATOR: &str = "lkjscript-deterministic-ed25519-tls-fixture-1";
const ROOT_ENVIRONMENT: &str = "LKJSCRIPT_OUTBOUND_HTTP_ROOT";
const MAXIMUM_COMMAND_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MAXIMUM_EXECUTABLE_BYTES: u64 = 384 * 1024 * 1024;
const MAXIMUM_RECEIPT_BYTES: u64 = 64 * 1024 * 1024;
const MAXIMUM_ORACLE_REQUEST_BYTES: usize = 64 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(90);
const RUNNER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_TIMEOUT: Duration = Duration::from_secs(35);
const KILL_TIMEOUT: Duration = Duration::from_secs(5);
const ORACLE_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(20);
const NIP11_DOCUMENT: &[u8] =
    b"{\"name\":\"local relay\",\"supported_nips\":[1,11],\"unknown\":{\"kept\":true}}\n";
const BAD_GATEWAY_BODY: &[u8] = b"bad gateway";
static RUN_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
struct Options {
    binary: PathBuf,
    evidence_root: Option<PathBuf>,
    machine: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SchemaIdentity {
    identity: String,
    version: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AcceptanceStatus {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ExecutableObservation {
    file: FileProof,
    byte_length: u64,
    mode: u32,
    sha256: String,
    verification_digest: VerificationDigest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AcceptanceReceipt {
    schema: SchemaIdentity,
    workflow: String,
    status: AcceptanceStatus,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: u128,
    elapsed_nanoseconds: u64,
    execution_context: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checkout_root: Option<String>,
    evidence_root: String,
    isolated_root: String,
    isolated_root_outside_checkout: bool,
    environment_names: Vec<String>,
    verifier: ExecutableObservation,
    candidate: ExecutableObservation,
    copied_candidate: ExecutableObservation,
    commands: Vec<CommandEvidence>,
    runners: Vec<RunnerEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<WorkflowResult>,
    cleanup: CleanupObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<Failure>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CommandEvidence {
    name: String,
    command: Vec<String>,
    expected: String,
    process: ProcessObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RunnerEvidence {
    name: String,
    command: Vec<String>,
    process: ProcessObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    ready: Option<ReadyObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stopped: Option<StoppedObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadyObservation {
    artifact_digest: String,
    target: String,
    runner: String,
    configured_listener: String,
    local_address: String,
    grants: BTreeMap<String, String>,
    elapsed_nanoseconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoppedObservation {
    admission_stopped: bool,
    remaining_tasks: u64,
    cleanup_failures: u64,
    elapsed_nanoseconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CompilerObservation {
    cache: String,
    compiled: u64,
    reused: u64,
    removed: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HttpObservation {
    name: String,
    expected: String,
    status: u16,
    body_bytes: u64,
    body_sha256: String,
    elapsed_nanoseconds: u64,
    upstream: WireObservation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WireObservation {
    connection_ordinal: u64,
    tls_established: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_line: Option<String>,
    headers: BTreeMap<String, String>,
    request_bytes: u64,
    request_sha256: String,
    response_bytes: u64,
    peer_closed_before_response: bool,
    elapsed_nanoseconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NoConnectionObservation {
    name: String,
    elapsed_nanoseconds: u64,
    connection_count_before: u64,
    connection_count_after: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CertificateObservation {
    generator: String,
    root_pem_bytes: u64,
    root_pem_sha256: String,
    root_der_sha256: String,
    leaf_der_sha256: String,
    expired_leaf_der_sha256: String,
    hostname: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NetworkResources {
    connections: u64,
    request_bytes: u64,
    response_bytes: u64,
    maximum_observed_request_bytes: u64,
    maximum_observed_response_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkflowResult {
    product_version: String,
    capabilities_digest: String,
    project: String,
    descriptor_path: String,
    artifact_path: String,
    repository: String,
    package: String,
    revision: String,
    semantic_state: String,
    semantic_root: String,
    owners: u64,
    dependencies: u64,
    targets: u64,
    tests: u64,
    check: CompilerObservation,
    incremental_build: CompilerObservation,
    clean_build: CompilerObservation,
    artifact_manifest: String,
    artifact_bundle: String,
    artifact_bytes: u64,
    artifact_sha256: String,
    clean_artifact_sha256: String,
    clean_incremental_equal: bool,
    descriptor: FileProof,
    artifact: FileProof,
    certificate: CertificateObservation,
    normalized_tls_endpoint: String,
    normalized_plaintext_endpoint: String,
    authority_before: AuthorityObservation,
    authority_after: AuthorityObservation,
    authority_unchanged: bool,
    responses: Vec<HttpObservation>,
    no_connection: Vec<NoConnectionObservation>,
    negative_cases: Vec<String>,
    restart_equal: bool,
    startup_failures_without_ready: u64,
    network: NetworkResources,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CleanupObservation {
    runner_cleanup_attempted: bool,
    runner_cleanup_complete: bool,
    oracle_cleanup_complete: bool,
    isolated_root_removed: bool,
    raw_secret_values_retained: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Failure {
    class: String,
    code: String,
    message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TransferredReceiptBinding {
    pub(crate) receipt_bytes: u64,
    pub(crate) receipt_sha256: String,
    pub(crate) verifier_sha256: String,
    pub(crate) candidate_sha256: String,
    pub(crate) elapsed_nanoseconds: u64,
    pub(crate) commands: u64,
    pub(crate) runners: u64,
    pub(crate) requests: u64,
    pub(crate) cleanup_complete: bool,
}

#[derive(Debug)]
struct AcceptanceFailure {
    class: &'static str,
    code: &'static str,
    message: String,
}

impl AcceptanceFailure {
    fn acceptance(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            class: "acceptance",
            code,
            message: message.into(),
        }
    }

    fn infrastructure(code: &'static str, error: impl std::fmt::Display) -> Self {
        Self {
            class: "infrastructure",
            code,
            message: error.to_string(),
        }
    }

    fn receipt(&self) -> Failure {
        Failure {
            class: self.class.to_owned(),
            code: self.code.to_owned(),
            message: self.message.clone(),
        }
    }
}

struct AcceptanceContext {
    observation_root: PathBuf,
    evidence_directory: PathBuf,
    copied_binary: PathBuf,
    root_pem: String,
    command_ordinal: u64,
    commands: Vec<CommandEvidence>,
    runners: Vec<RunnerEvidence>,
    active_runner: Option<ActiveRunner>,
}

struct ActiveRunner {
    name: String,
    command: Vec<String>,
    control: ProcessControl,
    receiver: Receiver<ProcessObservation>,
    thread: Option<thread::JoinHandle<()>>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    ready: Option<ReadyObservation>,
    terminal: Option<ProcessObservation>,
    started: Instant,
}

#[derive(Clone, Copy)]
enum ExpectedCommand {
    Success,
    CompactFailure(&'static str),
    RuntimeFailure,
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse_options(arguments)?;
    let verifier_path = current_verifier()?;
    let verifier = executable_observation(&verifier_path, "outbound HTTP verifier")?;
    let (execution_context, checkout_root, candidate_path, evidence_directory, observation_root) =
        if let Some(requested_evidence_root) = &options.evidence_root {
            let candidate = resolve_candidate(None, &options.binary)?;
            let evidence = create_explicit_evidence_root(requested_evidence_root)?;
            (
                "transferred".to_owned(),
                None,
                candidate,
                evidence.clone(),
                evidence,
            )
        } else {
            let repository = repository_root()?;
            let candidate = resolve_candidate(Some(&repository), &options.binary)?;
            let evidence = new_evidence_directory(&repository)?;
            (
                "contributor".to_owned(),
                Some(repository.clone()),
                candidate,
                evidence,
                repository,
            )
        };
    let receipt_path = evidence_directory.join("receipt.json");
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let candidate = executable_observation(&candidate_path, "outbound HTTP candidate")?;
    let temporary = tempfile::Builder::new()
        .prefix("lkjscript-outbound-http-")
        .tempdir()
        .map_err(|error| DevError::infrastructure(format!("create isolated workspace: {error}")))?;
    #[cfg(unix)]
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700))?;
    let isolated_root = temporary.path().canonicalize().map_err(|error| {
        DevError::infrastructure(format!("canonicalize isolated workspace: {error}"))
    })?;
    let outside_checkout = checkout_root
        .as_ref()
        .is_none_or(|repository| !isolated_root.starts_with(repository));
    let copied_binary = isolated_root.join("lkjscript");
    copy_binary(&candidate_path, &copied_binary)?;
    let copied_candidate = executable_observation(&copied_binary, "copied outbound candidate")?;
    if candidate.byte_length != copied_candidate.byte_length
        || candidate.sha256 != copied_candidate.sha256
        || candidate.verification_digest != copied_candidate.verification_digest
    {
        return Err(DevError::infrastructure(
            "copied outbound candidate does not match the selected candidate bytes",
        ));
    }

    let fixture = TlsFixture::generate().map_err(|error| {
        DevError::infrastructure(format!(
            "generate deterministic TLS fixture: {}",
            error.message
        ))
    })?;
    let mut context = AcceptanceContext {
        observation_root,
        evidence_directory: evidence_directory.clone(),
        copied_binary,
        root_pem: fixture.root_pem.clone(),
        command_ordinal: 0,
        commands: Vec::new(),
        runners: Vec::new(),
        active_runner: None,
    };
    let workflow = if outside_checkout {
        run_workflow(&mut context, &isolated_root, fixture)
    } else {
        Err(AcceptanceFailure::acceptance(
            "workspace_isolation",
            "temporary product workspace is inside the repository checkout",
        ))
    };
    let runner_cleanup_attempted = context.active_runner.is_some();
    let runner_cleanup = context.cleanup_runner();
    let mut failure = workflow.as_ref().err().map(AcceptanceFailure::receipt);
    if let Err(error) = &runner_cleanup
        && failure.is_none()
    {
        failure = Some(error.receipt());
    }
    let result = workflow.ok();
    let commands = std::mem::take(&mut context.commands);
    let runners = std::mem::take(&mut context.runners);
    drop(context);
    let isolated_root_text = isolated_root.display().to_string();
    let removal = temporary.close();
    let isolated_root_removed = removal.is_ok() && !isolated_root.exists();
    if let Err(error) = removal
        && failure.is_none()
    {
        failure = Some(Failure {
            class: "infrastructure".to_owned(),
            code: "workspace_cleanup".to_owned(),
            message: error.to_string(),
        });
    }
    let cleanup = CleanupObservation {
        runner_cleanup_attempted,
        runner_cleanup_complete: runner_cleanup.is_ok(),
        oracle_cleanup_complete: result.is_some(),
        isolated_root_removed,
        raw_secret_values_retained: false,
    };
    let status = if result.is_some()
        && failure.is_none()
        && cleanup.runner_cleanup_complete
        && cleanup.oracle_cleanup_complete
        && cleanup.isolated_root_removed
        && !cleanup.raw_secret_values_retained
    {
        AcceptanceStatus::Passed
    } else {
        AcceptanceStatus::Failed
    };
    let receipt = AcceptanceReceipt {
        schema: SchemaIdentity {
            identity: ACCEPTANCE_SCHEMA.to_owned(),
            version: ACCEPTANCE_SCHEMA_VERSION,
        },
        workflow: ACCEPTANCE_WORKFLOW.to_owned(),
        status,
        started_unix_nanoseconds: started_wall,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        execution_context,
        checkout_root: checkout_root.map(|path| path.display().to_string()),
        evidence_root: evidence_directory.display().to_string(),
        isolated_root: isolated_root_text,
        isolated_root_outside_checkout: outside_checkout,
        environment_names: vec!["LANG".to_owned(), ROOT_ENVIRONMENT.to_owned()],
        verifier,
        candidate,
        copied_candidate,
        commands,
        runners,
        result,
        cleanup,
        failure,
    };
    let published = evidence::publish_json(&receipt_path, &receipt)?;
    let receipt_sha256 = sha256_file(&receipt_path, MAXIMUM_RECEIPT_BYTES)?;
    print_summary(&options, &receipt, &published, &receipt_sha256)?;
    Ok(if status == AcceptanceStatus::Passed {
        0
    } else {
        1
    })
}

pub(crate) fn read_transferred_receipt(
    path: &Path,
    candidate_path: &Path,
    verifier_path: &Path,
) -> Result<TransferredReceiptBinding, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect transferred outbound HTTP receipt '{}': {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAXIMUM_RECEIPT_BYTES
    {
        return Err(DevError::corrupt(
            "transferred outbound HTTP receipt is unsafe or oversized",
        ));
    }
    let bytes = process::read_bounded(path, MAXIMUM_RECEIPT_BYTES)?;
    let receipt: AcceptanceReceipt = serde_json::from_slice(&bytes).map_err(|error| {
        DevError::corrupt(format!("decode transferred outbound HTTP receipt: {error}"))
    })?;
    if evidence::encode_json(&receipt)? != bytes {
        return Err(DevError::corrupt(
            "transferred outbound HTTP receipt is not canonical evidence encoding",
        ));
    }
    let expected_root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("transferred outbound receipt has no parent"))?
        .canonicalize()?;
    let verifier = executable_observation(verifier_path, "transferred outbound verifier")?;
    let candidate = executable_observation(candidate_path, "transferred outbound candidate")?;
    let result = receipt
        .result
        .as_ref()
        .ok_or_else(|| DevError::corrupt("passed outbound receipt omitted its result"))?;
    let cleanup_complete = receipt.cleanup.runner_cleanup_complete
        && receipt.cleanup.oracle_cleanup_complete
        && receipt.cleanup.isolated_root_removed
        && !receipt.cleanup.raw_secret_values_retained;
    if receipt.schema.identity != ACCEPTANCE_SCHEMA
        || receipt.schema.version != ACCEPTANCE_SCHEMA_VERSION
        || receipt.workflow != ACCEPTANCE_WORKFLOW
        || receipt.status != AcceptanceStatus::Passed
        || receipt.execution_context != "transferred"
        || receipt.checkout_root.is_some()
        || receipt.evidence_root != expected_root.display().to_string()
        || !receipt.isolated_root_outside_checkout
        || Path::new(&receipt.isolated_root).exists()
        || receipt.environment_names != ["LANG", ROOT_ENVIRONMENT]
        || receipt.verifier.sha256 != verifier.sha256
        || receipt.verifier.byte_length != verifier.byte_length
        || receipt.verifier.mode != verifier.mode
        || receipt.candidate.sha256 != candidate.sha256
        || receipt.candidate.byte_length != candidate.byte_length
        || receipt.candidate.mode != candidate.mode
        || receipt.copied_candidate.sha256 != candidate.sha256
        || receipt.failure.is_some()
        || !result.authority_unchanged
        || !result.clean_incremental_equal
        || !result.restart_equal
        || result.startup_failures_without_ready < 1
        || !cleanup_complete
    {
        return Err(DevError::corrupt(
            "transferred outbound HTTP receipt binding or acceptance mismatch",
        ));
    }
    Ok(TransferredReceiptBinding {
        receipt_bytes: metadata.len(),
        receipt_sha256: sha256_file(path, MAXIMUM_RECEIPT_BYTES)?,
        verifier_sha256: verifier.sha256,
        candidate_sha256: candidate.sha256,
        elapsed_nanoseconds: receipt.elapsed_nanoseconds,
        commands: receipt.commands.len() as u64,
        runners: receipt.runners.len() as u64,
        requests: result.responses.len() as u64,
        cleanup_complete,
    })
}

struct TlsFixture {
    root_pem: String,
    wrong_root_pem: String,
    server_config: Arc<ServerConfig>,
    expired_server_config: Arc<ServerConfig>,
    observation: CertificateObservation,
}

impl TlsFixture {
    fn generate() -> Result<Self, AcceptanceFailure> {
        let ca_key = deterministic_key(0x11)?;
        let mut ca_params = CertificateParams::new(Vec::<String>::new())
            .map_err(|error| AcceptanceFailure::infrastructure("certificate_params", error))?;
        ca_params.not_before = date_time_ymd(2025, 1, 1);
        ca_params.not_after = date_time_ymd(2040, 1, 1);
        ca_params.serial_number = Some(SerialNumber::from(vec![0x01]));
        ca_params.distinguished_name.remove(DnType::CommonName);
        ca_params
            .distinguished_name
            .push(DnType::CommonName, "lkjscript outbound oracle root");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let ca = CertifiedIssuer::self_signed(ca_params, ca_key)
            .map_err(|error| AcceptanceFailure::infrastructure("certificate_root", error))?;
        let root_pem = ca.pem();
        let root_der = ca.der().to_vec();

        let leaf_key = deterministic_key(0x22)?;
        let mut leaf_params = CertificateParams::new(vec!["localhost".to_owned()])
            .map_err(|error| AcceptanceFailure::infrastructure("certificate_params", error))?;
        leaf_params.not_before = date_time_ymd(2025, 1, 1);
        leaf_params.not_after = date_time_ymd(2035, 1, 1);
        leaf_params.serial_number = Some(SerialNumber::from(vec![0x02]));
        leaf_params.distinguished_name.remove(DnType::CommonName);
        leaf_params
            .distinguished_name
            .push(DnType::CommonName, "localhost");
        leaf_params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        leaf_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let leaf = leaf_params
            .signed_by(&leaf_key, &ca)
            .map_err(|error| AcceptanceFailure::infrastructure("certificate_leaf", error))?;
        let leaf_der = leaf.der().to_vec();
        let valid_server_config = server_config(&leaf_der, leaf_key.serialize_der())?;

        let expired_key = deterministic_key(0x33)?;
        let mut expired_params = CertificateParams::new(vec!["localhost".to_owned()])
            .map_err(|error| AcceptanceFailure::infrastructure("certificate_params", error))?;
        expired_params.not_before = date_time_ymd(2018, 1, 1);
        expired_params.not_after = date_time_ymd(2020, 1, 1);
        expired_params.serial_number = Some(SerialNumber::from(vec![0x03]));
        expired_params.distinguished_name.remove(DnType::CommonName);
        expired_params
            .distinguished_name
            .push(DnType::CommonName, "localhost");
        expired_params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        expired_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let expired_leaf = expired_params
            .signed_by(&expired_key, &ca)
            .map_err(|error| AcceptanceFailure::infrastructure("certificate_expired", error))?;
        let expired_leaf_der = expired_leaf.der().to_vec();
        let expired_server_config = server_config(&expired_leaf_der, expired_key.serialize_der())?;

        let wrong_key = deterministic_key(0x44)?;
        let mut wrong_params = CertificateParams::new(Vec::<String>::new())
            .map_err(|error| AcceptanceFailure::infrastructure("certificate_params", error))?;
        wrong_params.not_before = date_time_ymd(2025, 1, 1);
        wrong_params.not_after = date_time_ymd(2040, 1, 1);
        wrong_params.serial_number = Some(SerialNumber::from(vec![0x04]));
        wrong_params.distinguished_name.remove(DnType::CommonName);
        wrong_params
            .distinguished_name
            .push(DnType::CommonName, "lkjscript wrong outbound oracle root");
        wrong_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        wrong_params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        let wrong_root = wrong_params
            .self_signed(&wrong_key)
            .map_err(|error| AcceptanceFailure::infrastructure("certificate_wrong_root", error))?;
        let wrong_root_pem = wrong_root.pem();

        let observation = CertificateObservation {
            generator: FIXTURE_GENERATOR.to_owned(),
            root_pem_bytes: root_pem.len() as u64,
            root_pem_sha256: sha256_hex(root_pem.as_bytes()),
            root_der_sha256: sha256_hex(&root_der),
            leaf_der_sha256: sha256_hex(&leaf_der),
            expired_leaf_der_sha256: sha256_hex(&expired_leaf_der),
            hostname: "localhost".to_owned(),
        };
        Ok(Self {
            root_pem,
            wrong_root_pem,
            server_config: valid_server_config,
            expired_server_config,
            observation,
        })
    }
}

fn deterministic_key(fill: u8) -> Result<KeyPair, AcceptanceFailure> {
    // RFC 8410 OneAsymmetricKey prefix followed by a fixed Ed25519 seed. The generated keys and
    // Ed25519 signatures are deterministic; private DER is materialized only in the isolated
    // verifier process and is never written to tracked evidence.
    let mut der = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    der.extend(std::iter::repeat_n(fill, 32));
    let der = PrivatePkcs8KeyDer::from(der);
    KeyPair::from_pkcs8_der_and_sign_algo(&der, &PKCS_ED25519)
        .map_err(|error| AcceptanceFailure::infrastructure("certificate_key", error))
}

fn server_config(
    certificate: &[u8],
    private_key: Vec<u8>,
) -> Result<Arc<ServerConfig>, AcceptanceFailure> {
    let certificates = vec![CertificateDer::from(certificate.to_vec())];
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(private_key));
    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, key)
        .map(Arc::new)
        .map_err(|error| AcceptanceFailure::infrastructure("certificate_server", error))
}

#[derive(Clone)]
enum OracleTransport {
    Plaintext,
    Tls(Arc<ServerConfig>),
}

struct OracleScenario {
    response: Vec<u8>,
    delay: Duration,
}

impl OracleScenario {
    fn immediate(response: Vec<u8>) -> Self {
        Self {
            response,
            delay: Duration::ZERO,
        }
    }

    fn delayed(response: Vec<u8>, delay: Duration) -> Self {
        Self { response, delay }
    }
}

enum OracleCommand {
    Exchange {
        scenario: OracleScenario,
        started: SyncSender<()>,
        completed: SyncSender<Result<WireObservation, String>>,
    },
    ExpectNone {
        name: String,
        duration: Duration,
        completed: SyncSender<Result<NoConnectionObservation, String>>,
    },
    Shutdown(SyncSender<()>),
}

struct OracleTicket {
    started: Receiver<()>,
    completed: Receiver<Result<WireObservation, String>>,
}

impl OracleTicket {
    fn wait_started(&self) -> Result<(), AcceptanceFailure> {
        self.started
            .recv_timeout(ORACLE_TIMEOUT)
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_start", error))
    }

    fn wait(self) -> Result<WireObservation, AcceptanceFailure> {
        self.completed
            .recv_timeout(ORACLE_TIMEOUT)
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_result", error))?
            .map_err(|error| AcceptanceFailure::acceptance("oracle_exchange", error))
    }
}

struct RawOracle {
    address: SocketAddr,
    sender: mpsc::Sender<OracleCommand>,
    thread: Option<thread::JoinHandle<()>>,
}

impl RawOracle {
    fn start(transport: OracleTransport) -> Result<Self, AcceptanceFailure> {
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_bind", error))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_nonblocking", error))?;
        let address = listener
            .local_addr()
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_address", error))?;
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("outbound-http-raw-oracle".to_owned())
            .spawn(move || oracle_loop(listener, transport, receiver))
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_thread", error))?;
        Ok(Self {
            address,
            sender,
            thread: Some(thread),
        })
    }

    fn endpoint(&self, tls: bool, hostname: &str) -> String {
        let scheme = if tls { "https" } else { "http" };
        format!("{scheme}://{hostname}:{}/nip11", self.address.port())
    }

    fn enqueue(&self, scenario: OracleScenario) -> Result<OracleTicket, AcceptanceFailure> {
        let (started_sender, started) = mpsc::sync_channel(1);
        let (completed_sender, completed) = mpsc::sync_channel(1);
        self.sender
            .send(OracleCommand::Exchange {
                scenario,
                started: started_sender,
                completed: completed_sender,
            })
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_command", error))?;
        Ok(OracleTicket { started, completed })
    }

    fn expect_none(
        &self,
        name: &str,
        duration: Duration,
    ) -> Result<NoConnectionObservation, AcceptanceFailure> {
        let (sender, receiver) = mpsc::sync_channel(1);
        self.sender
            .send(OracleCommand::ExpectNone {
                name: name.to_owned(),
                duration,
                completed: sender,
            })
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_command", error))?;
        receiver
            .recv_timeout(duration.saturating_add(Duration::from_secs(2)))
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_no_connection", error))?
            .map_err(|error| AcceptanceFailure::acceptance("oracle_unexpected_connection", error))
    }

    fn shutdown(&mut self) -> Result<(), AcceptanceFailure> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        self.sender
            .send(OracleCommand::Shutdown(sender))
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_shutdown", error))?;
        receiver
            .recv_timeout(Duration::from_secs(2))
            .map_err(|error| AcceptanceFailure::infrastructure("oracle_shutdown", error))?;
        thread.join().map_err(|_| {
            AcceptanceFailure::infrastructure("oracle_thread", "oracle thread panicked")
        })
    }
}

impl Drop for RawOracle {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let (sender, receiver) = mpsc::sync_channel(1);
            let _ = self.sender.send(OracleCommand::Shutdown(sender));
            let _ = receiver.recv_timeout(Duration::from_secs(2));
            let _ = thread.join();
        }
    }
}

fn oracle_loop(
    listener: TcpListener,
    transport: OracleTransport,
    receiver: Receiver<OracleCommand>,
) {
    let mut connection_ordinal = 0_u64;
    while let Ok(command) = receiver.recv() {
        match command {
            OracleCommand::Exchange {
                scenario,
                started,
                completed,
            } => {
                let result = accept_bounded(&listener, ORACLE_TIMEOUT).and_then(|stream| {
                    connection_ordinal = connection_ordinal.saturating_add(1);
                    handle_oracle_connection(
                        stream,
                        &transport,
                        scenario,
                        connection_ordinal,
                        &started,
                    )
                });
                let _ = completed.send(result);
            }
            OracleCommand::ExpectNone {
                name,
                duration,
                completed,
            } => {
                let started = Instant::now();
                let before = connection_ordinal;
                let mut unexpected = None;
                while started.elapsed() < duration {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            connection_ordinal = connection_ordinal.saturating_add(1);
                            drop(stream);
                            unexpected = Some(format!(
                                "{name} unexpectedly opened outbound connection {connection_ordinal}"
                            ));
                            break;
                        }
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            thread::sleep(POLL_INTERVAL)
                        }
                        Err(error) => {
                            unexpected = Some(format!("{name} oracle accept failed: {error}"));
                            break;
                        }
                    }
                }
                let result = unexpected.map_or_else(
                    || {
                        Ok(NoConnectionObservation {
                            name,
                            elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
                            connection_count_before: before,
                            connection_count_after: connection_ordinal,
                        })
                    },
                    Err,
                );
                let _ = completed.send(result);
            }
            OracleCommand::Shutdown(completed) => {
                let _ = completed.send(());
                break;
            }
        }
    }
}

fn accept_bounded(listener: &TcpListener, timeout: Duration) -> Result<TcpStream, String> {
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, address)) => {
                if address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
                    return Err("raw oracle accepted a non-loopback peer".to_owned());
                }
                return Ok(stream);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if started.elapsed() >= timeout {
                    return Err("raw oracle timed out awaiting the expected connection".to_owned());
                }
                thread::sleep(POLL_INTERVAL);
            }
            Err(error) => return Err(format!("raw oracle accept failed: {error}")),
        }
    }
}

fn handle_oracle_connection(
    stream: TcpStream,
    transport: &OracleTransport,
    scenario: OracleScenario,
    connection_ordinal: u64,
    started_sender: &SyncSender<()>,
) -> Result<WireObservation, String> {
    let started = Instant::now();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .and_then(|()| stream.set_write_timeout(Some(Duration::from_secs(10))))
        .map_err(|error| format!("set raw oracle connection bounds: {error}"))?;
    match transport {
        OracleTransport::Plaintext => handle_oracle_stream(
            stream,
            scenario,
            connection_ordinal,
            false,
            started,
            started_sender,
        ),
        OracleTransport::Tls(config) => {
            let connection = ServerConnection::new(Arc::clone(config))
                .map_err(|error| format!("create raw TLS session: {error}"))?;
            let tls = StreamOwned::new(connection, stream);
            handle_oracle_stream(
                tls,
                scenario,
                connection_ordinal,
                true,
                started,
                started_sender,
            )
        }
    }
}

fn handle_oracle_stream<S: Read + Write>(
    mut stream: S,
    scenario: OracleScenario,
    connection_ordinal: u64,
    tls_expected: bool,
    started: Instant,
    started_sender: &SyncSender<()>,
) -> Result<WireObservation, String> {
    let request = match read_request_head(&mut stream) {
        Ok(request) => request,
        Err(error) => {
            let _ = started_sender.send(());
            return Ok(WireObservation {
                connection_ordinal,
                tls_established: false,
                request_line: None,
                headers: BTreeMap::new(),
                request_bytes: 0,
                request_sha256: sha256_hex(&[]),
                response_bytes: 0,
                peer_closed_before_response: true,
                elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
                failure: Some(redacted_transport_failure(&error)),
            });
        }
    };
    let (request_line, headers) = parse_raw_request(&request)?;
    let _ = started_sender.send(());
    if !scenario.delay.is_zero() {
        thread::sleep(scenario.delay);
    }
    let response_bytes = scenario.response.len() as u64;
    let write_result = stream
        .write_all(&scenario.response)
        .and_then(|()| stream.flush());
    Ok(WireObservation {
        connection_ordinal,
        tls_established: tls_expected,
        request_line: Some(request_line),
        headers,
        request_bytes: request.len() as u64,
        request_sha256: sha256_hex(&request),
        response_bytes,
        peer_closed_before_response: write_result.is_err(),
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        failure: write_result
            .err()
            .map(|error| redacted_transport_failure(&error)),
    })
}

fn read_request_head(stream: &mut impl Read) -> Result<Vec<u8>, io::Error> {
    let mut request = Vec::with_capacity(1024);
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "peer closed before a complete request",
            ));
        }
        if request.len().saturating_add(read) > MAXIMUM_ORACLE_REQUEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request exceeded oracle bound",
            ));
        }
        request.extend_from_slice(&buffer[..read]);
        if find_bytes(&request, b"\r\n\r\n").is_some() {
            return Ok(request);
        }
    }
}

fn parse_raw_request(bytes: &[u8]) -> Result<(String, BTreeMap<String, String>), String> {
    let end = find_bytes(bytes, b"\r\n\r\n")
        .ok_or_else(|| "raw request omitted header terminator".to_owned())?;
    if end.saturating_add(4) != bytes.len() {
        return Err("raw GET request unexpectedly carried a body".to_owned());
    }
    let text = std::str::from_utf8(&bytes[..end])
        .map_err(|_| "raw request head was not UTF-8".to_owned())?;
    let mut lines = text.split("\r\n");
    let request_line = lines
        .next()
        .filter(|line| !line.is_empty())
        .ok_or_else(|| "raw request omitted request line".to_owned())?
        .to_owned();
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| "raw request contained malformed header".to_owned())?;
        let name = name.to_ascii_lowercase();
        if name.is_empty() || headers.insert(name, value.trim().to_owned()).is_some() {
            return Err("raw request contained empty or duplicate header".to_owned());
        }
    }
    Ok((request_line, headers))
}

fn redacted_transport_failure(error: &impl std::fmt::Display) -> String {
    let text = error.to_string();
    if text.to_ascii_lowercase().contains("certificate") {
        "tls_peer_rejected".to_owned()
    } else {
        "peer_closed_or_protocol_failed".to_owned()
    }
}

struct ObservedCommand {
    stdout: Vec<u8>,
}

impl AcceptanceContext {
    fn invoke(
        &mut self,
        name: &str,
        arguments: Vec<String>,
        cwd: &Path,
        expected: ExpectedCommand,
    ) -> Result<ObservedCommand, AcceptanceFailure> {
        let safe_name = safe_name(name)?;
        let ordinal = self.command_ordinal;
        self.command_ordinal = self.command_ordinal.saturating_add(1);
        let stdout_path = self
            .evidence_directory
            .join(format!("{ordinal:03}-{safe_name}.stdout.log"));
        let stderr_path = self
            .evidence_directory
            .join(format!("{ordinal:03}-{safe_name}.stderr.log"));
        let mut command = vec![self.copied_binary.display().to_string()];
        command.extend(arguments);
        let specification = ProcessSpec {
            command: command.clone(),
            cwd: cwd.to_path_buf(),
            environment: isolated_environment(Some(&self.root_pem)),
            timeout: COMMAND_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
            stdout_path: stdout_path.clone(),
            stderr_path,
            unavailable_exit_code: None,
        };
        let observation = process::run(&specification, &self.observation_root);
        let stdout = process::read_bounded(&stdout_path, MAXIMUM_COMMAND_OUTPUT_BYTES)
            .map_err(|error| AcceptanceFailure::infrastructure("command_stdout", error))?;
        let mut diagnostic_code = None;
        let expected_text = match expected {
            ExpectedCommand::Success => {
                if observation.status != ProcessStatus::Passed {
                    return Err(AcceptanceFailure::acceptance(
                        "command_status",
                        format!("{name} did not succeed"),
                    ));
                }
                "success"
            }
            ExpectedCommand::CompactFailure(code) => {
                if observation.status == ProcessStatus::Passed {
                    return Err(AcceptanceFailure::acceptance(
                        "command_failure_status",
                        format!("{name} unexpectedly succeeded"),
                    ));
                }
                let records = compact_records(name, &stdout)?;
                let observed = required_field(required_record(&records, "diagnostic")?, "code")?;
                if observed != code {
                    return Err(AcceptanceFailure::acceptance(
                        "command_failure_code",
                        format!("{name} returned {observed} instead of {code}"),
                    ));
                }
                diagnostic_code = Some(observed.to_owned());
                "compact-failure"
            }
            ExpectedCommand::RuntimeFailure => {
                if observation.status == ProcessStatus::Passed
                    || stdout
                        .windows(b"\"event\":\"ready\"".len())
                        .any(|window| window == b"\"event\":\"ready\"")
                {
                    return Err(AcceptanceFailure::acceptance(
                        "runtime_failure_ready",
                        format!("{name} succeeded or emitted readiness"),
                    ));
                }
                let value: Value = serde_json::from_slice(&stdout).map_err(|error| {
                    AcceptanceFailure::infrastructure("runtime_failure_json", error)
                })?;
                diagnostic_code = value
                    .pointer("/error/code")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                if diagnostic_code.is_none() {
                    return Err(AcceptanceFailure::acceptance(
                        "runtime_failure_shape",
                        format!("{name} omitted its redacted error code"),
                    ));
                }
                "startup-failure-without-ready"
            }
        };
        self.commands.push(CommandEvidence {
            name: name.to_owned(),
            command,
            expected: expected_text.to_owned(),
            process: observation.clone(),
            diagnostic_code,
        });
        Ok(ObservedCommand { stdout })
    }

    fn start_runner(
        &mut self,
        name: &str,
        arguments: Vec<String>,
        cwd: &Path,
    ) -> Result<ReadyObservation, AcceptanceFailure> {
        let root_pem = self.root_pem.clone();
        self.start_runner_with_root(name, arguments, cwd, &root_pem)
    }

    fn start_runner_with_root(
        &mut self,
        name: &str,
        arguments: Vec<String>,
        cwd: &Path,
        root_pem: &str,
    ) -> Result<ReadyObservation, AcceptanceFailure> {
        if self.active_runner.is_some() {
            return Err(AcceptanceFailure::acceptance(
                "runner_overlap",
                "acceptance attempted to start two resident processes",
            ));
        }
        let safe_name = safe_name(name)?;
        let stdout_path = self
            .evidence_directory
            .join(format!("runner-{safe_name}.stdout.log"));
        let stderr_path = self
            .evidence_directory
            .join(format!("runner-{safe_name}.stderr.log"));
        let mut command = vec![self.copied_binary.display().to_string()];
        command.extend(arguments);
        let specification = ProcessSpec {
            command: command.clone(),
            cwd: cwd.to_path_buf(),
            environment: isolated_environment(Some(root_pem)),
            timeout: RUNNER_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
            stdout_path: stdout_path.clone(),
            stderr_path: stderr_path.clone(),
            unavailable_exit_code: None,
        };
        let observation_root = self.observation_root.clone();
        let control = ProcessControl::default();
        let child_control = control.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(format!("outbound-http-{safe_name}"))
            .spawn(move || {
                let observation =
                    process::run_controlled(&specification, &observation_root, &child_control);
                let _ = sender.send(observation);
            })
            .map_err(|error| AcceptanceFailure::infrastructure("runner_thread", error))?;
        self.active_runner = Some(ActiveRunner {
            name: name.to_owned(),
            command,
            control,
            receiver,
            thread: Some(thread),
            stdout_path,
            stderr_path,
            ready: None,
            terminal: None,
            started: Instant::now(),
        });
        let ready = self
            .active_runner
            .as_mut()
            .ok_or_else(|| AcceptanceFailure::acceptance("runner_missing", "runner vanished"))?
            .wait_ready()?;
        self.active_runner
            .as_mut()
            .ok_or_else(|| AcceptanceFailure::acceptance("runner_missing", "runner vanished"))?
            .ready = Some(ready.clone());
        Ok(ready)
    }

    fn stop_runner(&mut self) -> Result<(), AcceptanceFailure> {
        let mut runner = self.active_runner.take().ok_or_else(|| {
            AcceptanceFailure::acceptance("runner_missing", "no active runner to stop")
        })?;
        runner.control.interrupt();
        let observation = match runner.wait_terminal(STOP_TIMEOUT) {
            Ok(observation) => observation,
            Err(error) => {
                runner.control.kill();
                let _ = runner.wait_terminal(KILL_TIMEOUT);
                runner.join()?;
                return Err(error);
            }
        };
        runner.join()?;
        let stdout = process::read_bounded(&runner.stdout_path, MAXIMUM_COMMAND_OUTPUT_BYTES)
            .map_err(|error| AcceptanceFailure::infrastructure("runner_stdout", error))?;
        let stopped = parse_stopped(&stdout)?;
        self.runners.push(RunnerEvidence {
            name: runner.name,
            command: runner.command,
            process: observation.clone(),
            ready: runner.ready,
            stopped: Some(stopped.clone()),
        });
        if observation.status != ProcessStatus::Passed
            || !stopped.admission_stopped
            || stopped.remaining_tasks != 0
            || stopped.cleanup_failures != 0
        {
            return Err(AcceptanceFailure::acceptance(
                "runner_shutdown",
                "resident process did not stop cleanly",
            ));
        }
        Ok(())
    }

    fn cleanup_runner(&mut self) -> Result<(), AcceptanceFailure> {
        let Some(mut runner) = self.active_runner.take() else {
            return Ok(());
        };
        runner.control.kill();
        let observation = runner.wait_terminal(KILL_TIMEOUT)?;
        runner.join()?;
        self.runners.push(RunnerEvidence {
            name: runner.name,
            command: runner.command,
            process: observation,
            ready: runner.ready,
            stopped: None,
        });
        Ok(())
    }
}

impl ActiveRunner {
    fn wait_ready(&mut self) -> Result<ReadyObservation, AcceptanceFailure> {
        let started = Instant::now();
        loop {
            if let Some(line) = first_line(&self.stdout_path)? {
                let mut ready = parse_ready(&line)?;
                ready.elapsed_nanoseconds = duration_nanoseconds(self.started.elapsed());
                return Ok(ready);
            }
            if self.poll_terminal()?.is_some() {
                return Err(AcceptanceFailure::acceptance(
                    "runner_before_ready",
                    "resident process exited before readiness",
                ));
            }
            if started.elapsed() >= READY_TIMEOUT {
                return Err(AcceptanceFailure::acceptance(
                    "runner_ready_timeout",
                    "resident process omitted readiness before the deadline",
                ));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn poll_terminal(&mut self) -> Result<Option<&ProcessObservation>, AcceptanceFailure> {
        if self.terminal.is_none() {
            match self.receiver.try_recv() {
                Ok(observation) => self.terminal = Some(observation),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    return Err(AcceptanceFailure::infrastructure(
                        "runner_channel",
                        "resident process observation channel disconnected",
                    ));
                }
            }
        }
        Ok(self.terminal.as_ref())
    }

    fn wait_terminal(
        &mut self,
        timeout: Duration,
    ) -> Result<ProcessObservation, AcceptanceFailure> {
        if let Some(observation) = self.terminal.take() {
            return Ok(observation);
        }
        self.receiver
            .recv_timeout(timeout)
            .map_err(|error| AcceptanceFailure::infrastructure("runner_stop_timeout", error))
    }

    fn join(&mut self) -> Result<(), AcceptanceFailure> {
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| {
                AcceptanceFailure::infrastructure("runner_thread", "runner thread panicked")
            })?;
        }
        let stderr = process::read_bounded(&self.stderr_path, MAXIMUM_COMMAND_OUTPUT_BYTES)
            .map_err(|error| AcceptanceFailure::infrastructure("runner_stderr", error))?;
        if !stderr.is_empty() {
            return Err(AcceptanceFailure::acceptance(
                "runner_stderr",
                "classified resident workflow wrote to stderr",
            ));
        }
        Ok(())
    }
}

fn run_workflow(
    context: &mut AcceptanceContext,
    isolated_root: &Path,
    fixture: TlsFixture,
) -> Result<WorkflowResult, AcceptanceFailure> {
    let mut tls_oracle =
        RawOracle::start(OracleTransport::Tls(Arc::clone(&fixture.server_config)))?;
    let mut expired_oracle = RawOracle::start(OracleTransport::Tls(Arc::clone(
        &fixture.expired_server_config,
    )))?;
    let mut plaintext_oracle = RawOracle::start(OracleTransport::Plaintext)?;
    let tls_endpoint = tls_oracle.endpoint(true, "localhost");
    let plaintext_endpoint = plaintext_oracle.endpoint(false, "127.0.0.1");
    let relay_locator = tls_endpoint.replacen("https://", "wss://", 1);
    let project = isolated_root.join("application");
    let descriptor = project.join("service.deployment.json");
    let artifact = project.join("generated/application.lkja");
    let clean_artifact = project.join("generated/application-clean.lkja");

    let capabilities = context.invoke(
        "capabilities",
        vec!["capabilities".to_owned()],
        isolated_root,
        ExpectedCommand::Success,
    )?;
    let capability_records = compact_records("capabilities", &capabilities.stdout)?;
    let product = required_record(&capability_records, "product")?;
    if required_field(product, "name")? != "lkjscript"
        || required_field(product, "version")? != "0.1.15"
    {
        return Err(AcceptanceFailure::acceptance(
            "capabilities_product",
            "candidate did not advertise exact lkjscript 0.1.15 source",
        ));
    }
    let product_version = required_field(product, "version")?.to_owned();
    let capabilities_digest = required_field(
        required_record(&capability_records, "capabilities")?,
        "digest",
    )?
    .to_owned();
    let new_discovery = context.invoke(
        "capabilities-new",
        vec!["capabilities".to_owned(), "new".to_owned()],
        isolated_root,
        ExpectedCommand::Success,
    )?;
    let template_discovery = context.invoke(
        "capabilities-templates",
        vec![
            "capabilities".to_owned(),
            "--section".to_owned(),
            "templates".to_owned(),
        ],
        isolated_root,
        ExpectedCommand::Success,
    )?;
    let mut discovery_records = compact_records("new discovery", &new_discovery.stdout)?;
    discovery_records.extend(compact_records(
        "template discovery",
        &template_discovery.stdout,
    )?);
    require_template_discovery(&discovery_records)?;

    for (name, arguments, code) in [
        (
            "new-missing-relay-url",
            vec![
                "new".to_owned(),
                isolated_root.join("missing").display().to_string(),
                "--template".to_owned(),
                "nostr-relay-info".to_owned(),
            ],
            "cli_usage",
        ),
        (
            "new-foreign-relay-option",
            vec![
                "new".to_owned(),
                isolated_root.join("foreign").display().to_string(),
                "--template".to_owned(),
                "command".to_owned(),
                "--relay-url".to_owned(),
                relay_locator.clone(),
            ],
            "cli_usage",
        ),
        (
            "new-malformed-relay-url",
            vec![
                "new".to_owned(),
                isolated_root.join("malformed").display().to_string(),
                "--template".to_owned(),
                "nostr-relay-info".to_owned(),
                "--relay-url".to_owned(),
                format!("{relay_locator}?credential=forbidden"),
            ],
            "http_client_endpoint",
        ),
    ] {
        context.invoke(
            name,
            arguments,
            isolated_root,
            ExpectedCommand::CompactFailure(code),
        )?;
    }
    for path in ["missing", "foreign", "malformed"] {
        if isolated_root.join(path).exists() {
            return Err(AcceptanceFailure::acceptance(
                "new_partial_visibility",
                format!("rejected {path} project became visible"),
            ));
        }
    }

    let created = context.invoke(
        "new-nostr-relay-info",
        vec![
            "new".to_owned(),
            project.display().to_string(),
            "--template".to_owned(),
            "nostr-relay-info".to_owned(),
            "--name".to_owned(),
            "application".to_owned(),
            "--relay-url".to_owned(),
            relay_locator,
        ],
        isolated_root,
        ExpectedCommand::Success,
    )?;
    let created_records = compact_records("new nostr relay info", &created.stdout)?;
    require_field(&created_records, "project", "template", "nostr-relay-info")?;
    let repository =
        required_field(required_record(&created_records, "repository")?, "id")?.to_owned();
    let package = required_field(required_record(&created_records, "package")?, "id")?.to_owned();
    let revision = required_field(required_record(&created_records, "revision")?, "id")?.to_owned();
    let semantic_state =
        required_field(required_record(&created_records, "state")?, "digest")?.to_owned();
    let semantic_root =
        required_field(required_record(&created_records, "root")?, "digest")?.to_owned();
    let summary = required_record(&created_records, "summary")?;
    let owners = parse_u64(required_field(summary, "owners")?, "owner count")?;
    let dependencies = parse_u64(required_field(summary, "dependencies")?, "dependency count")?;
    let targets = parse_u64(required_field(summary, "targets")?, "target count")?;
    let tests = parse_u64(required_field(summary, "tests")?, "test count")?;
    if owners == 0 || dependencies != 1 || targets != 1 || tests != 2 {
        return Err(AcceptanceFailure::acceptance(
            "created_topology",
            "Nostr relay information recipe topology is not the closed maintained shape",
        ));
    }
    require_field(&created_records, "deployment", "target", "serve")?;
    require_field(&created_records, "deployment", "runner", "http")?;
    require_next_actions(&created_records)?;

    context.invoke(
        "status",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "status".to_owned(),
        ],
        isolated_root,
        ExpectedCommand::Success,
    )?;
    context.invoke(
        "query-http-client",
        vec![
            "package".to_owned(),
            "builtin".to_owned(),
            "query".to_owned(),
            "owners".to_owned(),
            "--name".to_owned(),
            "HttpClient".to_owned(),
        ],
        isolated_root,
        ExpectedCommand::Success,
    )?;

    update_descriptor(
        &descriptor,
        DescriptorPolicy {
            endpoint: tls_endpoint.clone(),
            address_policy: "loopback_only",
            trust: "named_pem_root",
            maximum_response_headers: 8,
            maximum_response_header_bytes: 1024,
            maximum_response_body_bytes: 1024,
            maximum_concurrent_requests: 1,
            connection_timeout_milliseconds: 200,
            total_timeout_milliseconds: 300,
            cleanup_timeout_milliseconds: 1000,
        },
    )?;
    let checked = context.invoke(
        "check",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "check".to_owned(),
        ],
        isolated_root,
        ExpectedCommand::Success,
    )?;
    let check_records = compact_records("check", &checked.stdout)?;
    let check = compiler_observation(&check_records)?;
    let test_record = required_record(&check_records, "tests")?;
    if required_field(test_record, "failed")? != "0"
        || required_field(test_record, "differential")? != "equal"
    {
        return Err(AcceptanceFailure::acceptance(
            "graph_tests",
            "Nostr relay information graph tests did not pass differentially",
        ));
    }

    let built = context.invoke(
        "build-incremental",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "build".to_owned(),
            "--output".to_owned(),
            artifact.display().to_string(),
        ],
        isolated_root,
        ExpectedCommand::Success,
    )?;
    let build_records = compact_records("incremental build", &built.stdout)?;
    let incremental_build = compiler_observation(&build_records)?;
    let artifact_record = required_record(&build_records, "artifact")?;
    let artifact_manifest = required_field(artifact_record, "manifest")?.to_owned();
    let artifact_bundle = required_field(artifact_record, "bundle")?.to_owned();
    let artifact_bytes = fs::read(&artifact)
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_read", error))?;
    if artifact_bytes.len() as u64 > MAXIMUM_ARTIFACT_BYTES {
        return Err(AcceptanceFailure::acceptance(
            "artifact_bound",
            "Nostr application artifact exceeded acceptance bound",
        ));
    }
    let artifact_sha256 = sha256_hex(&artifact_bytes);
    let derived = project.join("derived");
    if derived.exists() {
        fs::remove_dir_all(&derived)
            .map_err(|error| AcceptanceFailure::infrastructure("derived_reset", error))?;
    }
    let clean = context.invoke(
        "build-clean",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "build".to_owned(),
            "--output".to_owned(),
            clean_artifact.display().to_string(),
        ],
        isolated_root,
        ExpectedCommand::Success,
    )?;
    let clean_records = compact_records("clean build", &clean.stdout)?;
    let clean_build = compiler_observation(&clean_records)?;
    let clean_bytes = fs::read(&clean_artifact)
        .map_err(|error| AcceptanceFailure::infrastructure("clean_artifact_read", error))?;
    let clean_artifact_sha256 = sha256_hex(&clean_bytes);
    let clean_incremental_equal = clean_bytes == artifact_bytes;
    if !clean_incremental_equal {
        return Err(AcceptanceFailure::acceptance(
            "artifact_determinism",
            "clean and incremental Nostr application artifacts disagree",
        ));
    }
    let authority_before = authority::observe_graph_authority(&project)
        .map_err(|error| AcceptanceFailure::infrastructure("authority_before", error))?;

    let mut responses = Vec::new();
    let mut no_connection = Vec::new();
    let mut negative_cases = Vec::new();
    let first = exercise_primary_tls(
        context,
        isolated_root,
        &descriptor,
        &artifact_bundle,
        &tls_endpoint,
        &mut tls_oracle,
        &mut responses,
        &mut negative_cases,
    )?;
    let restart = run_valid_once(
        context,
        isolated_root,
        &descriptor,
        &artifact_bundle,
        &mut tls_oracle,
        "trusted-tls-restart",
    )?;
    let restart_equal = first.status == restart.status
        && first.body_sha256 == restart.body_sha256
        && first.upstream.request_sha256 == restart.upstream.request_sha256;
    if !restart_equal {
        return Err(AcceptanceFailure::acceptance(
            "restart_response",
            "restarted outbound application response or wire request changed",
        ));
    }
    responses.push(restart);

    exercise_plaintext(
        context,
        isolated_root,
        &descriptor,
        &artifact_bundle,
        &plaintext_endpoint,
        &mut plaintext_oracle,
        &mut responses,
    )?;
    exercise_destination_and_tls_failures(
        context,
        isolated_root,
        &descriptor,
        &artifact_bundle,
        &tls_endpoint,
        &fixture,
        &mut tls_oracle,
        &mut expired_oracle,
        &mut responses,
        &mut no_connection,
        &mut negative_cases,
    )?;
    let startup_failures_without_ready =
        exercise_startup_failure(context, isolated_root, &descriptor, &tls_endpoint)?;

    update_descriptor(
        &descriptor,
        DescriptorPolicy::trusted_tls(tls_endpoint.clone()),
    )?;
    let authority_after = authority::observe_graph_authority(&project)
        .map_err(|error| AcceptanceFailure::infrastructure("authority_after", error))?;
    let authority_unchanged = authority_before == authority_after;
    if !authority_unchanged {
        return Err(AcceptanceFailure::acceptance(
            "authority_changed",
            format!(
                "checking, building, transport attempts, cancellation, or shutdown changed graph authority (before={:?}, after={:?})",
                authority_before, authority_after
            ),
        ));
    }

    tls_oracle.shutdown()?;
    expired_oracle.shutdown()?;
    plaintext_oracle.shutdown()?;
    let network = network_resources(&responses);
    let descriptor_proof = evidence::proof(&descriptor, descriptor.display().to_string())
        .map_err(|error| AcceptanceFailure::infrastructure("descriptor_proof", error))?;
    let artifact_proof = evidence::proof(&artifact, artifact.display().to_string())
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_proof", error))?;
    Ok(WorkflowResult {
        product_version,
        capabilities_digest,
        project: project.display().to_string(),
        descriptor_path: descriptor.display().to_string(),
        artifact_path: artifact.display().to_string(),
        repository,
        package,
        revision,
        semantic_state,
        semantic_root,
        owners,
        dependencies,
        targets,
        tests,
        check,
        incremental_build,
        clean_build,
        artifact_manifest,
        artifact_bundle,
        artifact_bytes: artifact_bytes.len() as u64,
        artifact_sha256,
        clean_artifact_sha256,
        clean_incremental_equal,
        descriptor: descriptor_proof,
        artifact: artifact_proof,
        certificate: fixture.observation,
        normalized_tls_endpoint: tls_endpoint,
        normalized_plaintext_endpoint: plaintext_endpoint,
        authority_before,
        authority_after,
        authority_unchanged,
        responses,
        no_connection,
        negative_cases,
        restart_equal,
        startup_failures_without_ready,
        network,
    })
}

#[derive(Clone)]
struct DescriptorPolicy {
    endpoint: String,
    address_policy: &'static str,
    trust: &'static str,
    maximum_response_headers: u64,
    maximum_response_header_bytes: u64,
    maximum_response_body_bytes: u64,
    maximum_concurrent_requests: u64,
    connection_timeout_milliseconds: u64,
    total_timeout_milliseconds: u64,
    cleanup_timeout_milliseconds: u64,
}

impl DescriptorPolicy {
    fn trusted_tls(endpoint: String) -> Self {
        Self {
            endpoint,
            address_policy: "loopback_only",
            trust: "named_pem_root",
            maximum_response_headers: 8,
            maximum_response_header_bytes: 1024,
            maximum_response_body_bytes: 1024,
            maximum_concurrent_requests: 1,
            connection_timeout_milliseconds: 200,
            total_timeout_milliseconds: 300,
            cleanup_timeout_milliseconds: 1000,
        }
    }
}

fn update_descriptor(path: &Path, policy: DescriptorPolicy) -> Result<(), AcceptanceFailure> {
    let bytes = fs::read(path)
        .map_err(|error| AcceptanceFailure::infrastructure("descriptor_read", error))?;
    let mut value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| AcceptanceFailure::infrastructure("descriptor_json", error))?;
    let object = value.as_object_mut().ok_or_else(|| {
        AcceptanceFailure::acceptance("descriptor_shape", "deployment descriptor is not an object")
    })?;
    object.insert(
        "secrets".to_owned(),
        serde_json::json!([{"name": "relay-root", "variable": ROOT_ENVIRONMENT}]),
    );
    let grants = object
        .get_mut("grants")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            AcceptanceFailure::acceptance(
                "descriptor_shape",
                "deployment descriptor omitted grants",
            )
        })?;
    let adapter = grants
        .iter_mut()
        .find(|grant| grant.get("requirement").and_then(Value::as_str) == Some("relay"))
        .and_then(|grant| grant.get_mut("adapter"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            AcceptanceFailure::acceptance(
                "descriptor_shape",
                "deployment descriptor omitted relay HTTP client adapter",
            )
        })?;
    adapter.insert("endpoint".to_owned(), Value::String(policy.endpoint));
    adapter.insert(
        "address_policy".to_owned(),
        Value::String(policy.address_policy.to_owned()),
    );
    adapter.insert(
        "trust".to_owned(),
        if policy.trust == "named_pem_root" {
            serde_json::json!({"kind": "named_pem_root", "secret": "relay-root"})
        } else {
            serde_json::json!({"kind": "webpki_roots"})
        },
    );
    let limits = adapter
        .get_mut("limits")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            AcceptanceFailure::acceptance(
                "descriptor_shape",
                "HTTP client adapter omitted independent limits",
            )
        })?;
    for (name, number) in [
        ("maximum_request_headers", 8),
        ("maximum_request_header_bytes", 1024),
        ("maximum_response_headers", policy.maximum_response_headers),
        (
            "maximum_response_header_bytes",
            policy.maximum_response_header_bytes,
        ),
        (
            "maximum_response_body_bytes",
            policy.maximum_response_body_bytes,
        ),
        ("maximum_dns_results", 4),
        (
            "maximum_concurrent_requests",
            policy.maximum_concurrent_requests,
        ),
        (
            "connection_timeout_milliseconds",
            policy.connection_timeout_milliseconds,
        ),
        (
            "total_timeout_milliseconds",
            policy.total_timeout_milliseconds,
        ),
        (
            "cleanup_timeout_milliseconds",
            policy.cleanup_timeout_milliseconds,
        ),
    ] {
        limits.insert(name.to_owned(), Value::from(number));
    }
    let mut encoded = serde_json::to_vec_pretty(&value)
        .map_err(|error| AcceptanceFailure::infrastructure("descriptor_encode", error))?;
    encoded.push(b'\n');
    fs::write(path, encoded)
        .map_err(|error| AcceptanceFailure::infrastructure("descriptor_publish", error))
}

#[derive(Clone, Copy)]
enum ResponseExpectation<'a> {
    Success,
    GraphGateway(&'a [u8]),
    CapabilityGateway(&'a [u8]),
}

#[allow(clippy::too_many_arguments)]
fn exercise_primary_tls(
    context: &mut AcceptanceContext,
    cwd: &Path,
    descriptor: &Path,
    artifact_bundle: &str,
    endpoint: &str,
    oracle: &mut RawOracle,
    responses: &mut Vec<HttpObservation>,
    negative_cases: &mut Vec<String>,
) -> Result<HttpObservation, AcceptanceFailure> {
    let ready = start_exact_runner(
        context,
        "trusted-tls-primary",
        cwd,
        descriptor,
        artifact_bundle,
        None,
    )?;
    let address = ready_address(&ready)?;
    let valid_response = raw_response(
        200,
        "OK",
        &[
            ("Content-Type", "Application/Nostr+Json; charset=utf-8"),
            ("X-Relay-Oracle", "raw"),
        ],
        NIP11_DOCUMENT,
    );
    let first = run_exchange(
        address,
        endpoint,
        oracle,
        "trusted-tls-valid",
        OracleScenario::immediate(valid_response.clone()),
        ResponseExpectation::Success,
    )?;
    responses.push(first.clone());

    let non_200_marker = b"remote-private-non-200-body";
    responses.push(run_exchange(
        address,
        endpoint,
        oracle,
        "non-200",
        OracleScenario::immediate(raw_response(
            503,
            "Unavailable",
            &[("Content-Type", "application/nostr+json")],
            non_200_marker,
        )),
        ResponseExpectation::GraphGateway(non_200_marker),
    )?);
    negative_cases.push("non_200_to_local_502".to_owned());

    let wrong_media_marker = b"remote-private-wrong-media-body";
    responses.push(run_exchange(
        address,
        endpoint,
        oracle,
        "wrong-media-type",
        OracleScenario::immediate(raw_response(
            200,
            "OK",
            &[("Content-Type", "application/json")],
            wrong_media_marker,
        )),
        ResponseExpectation::GraphGateway(wrong_media_marker),
    )?);
    negative_cases.push("wrong_media_type_to_local_502".to_owned());

    let sentinel = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
        .map_err(|error| AcceptanceFailure::infrastructure("redirect_sentinel", error))?;
    sentinel
        .set_nonblocking(true)
        .map_err(|error| AcceptanceFailure::infrastructure("redirect_sentinel", error))?;
    let location = format!(
        "http://127.0.0.1:{}/redirect-must-not-be-followed",
        sentinel
            .local_addr()
            .map_err(|error| AcceptanceFailure::infrastructure("redirect_sentinel", error))?
            .port()
    );
    responses.push(run_exchange(
        address,
        endpoint,
        oracle,
        "redirect-not-followed",
        OracleScenario::immediate(raw_response(
            302,
            "Found",
            &[
                ("Content-Type", "application/nostr+json"),
                ("Location", &location),
            ],
            b"remote-redirect-body",
        )),
        ResponseExpectation::GraphGateway(b"remote-redirect-body"),
    )?);
    thread::sleep(Duration::from_millis(100));
    match sentinel.accept() {
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
        Ok(_) => {
            return Err(AcceptanceFailure::acceptance(
                "redirect_followed",
                "HTTP client followed a forbidden redirect",
            ));
        }
        Err(error) => {
            return Err(AcceptanceFailure::infrastructure(
                "redirect_sentinel",
                error,
            ));
        }
    }
    negative_cases.push("redirect_not_followed".to_owned());

    let many_headers = (0..12)
        .map(|index| (format!("X-Header-{index}"), "value".to_owned()))
        .collect::<Vec<_>>();
    responses.push(run_exchange(
        address,
        endpoint,
        oracle,
        "response-header-count-exhaustion",
        OracleScenario::immediate(raw_response_owned_headers(
            200,
            "OK",
            many_headers,
            NIP11_DOCUMENT,
        )),
        ResponseExpectation::CapabilityGateway(b"header-count-marker"),
    )?);
    negative_cases.push("response_header_count_exhaustion".to_owned());

    let oversized_header = "x".repeat(1400);
    responses.push(run_exchange(
        address,
        endpoint,
        oracle,
        "response-header-byte-exhaustion",
        OracleScenario::immediate(raw_response(
            200,
            "OK",
            &[
                ("Content-Type", "application/nostr+json"),
                ("X-Oversized", &oversized_header),
            ],
            NIP11_DOCUMENT,
        )),
        ResponseExpectation::CapabilityGateway(oversized_header.as_bytes()),
    )?);
    negative_cases.push("response_header_byte_exhaustion".to_owned());

    let oversized_body = vec![b'z'; 2048];
    responses.push(run_exchange(
        address,
        endpoint,
        oracle,
        "response-body-exhaustion",
        OracleScenario::immediate(raw_response(
            200,
            "OK",
            &[("Content-Type", "application/nostr+json")],
            &oversized_body,
        )),
        ResponseExpectation::CapabilityGateway(&oversized_body),
    )?);
    negative_cases.push("response_body_exhaustion".to_owned());

    responses.push(run_exchange(
        address,
        endpoint,
        oracle,
        "delayed-response-timeout",
        OracleScenario::delayed(valid_response.clone(), Duration::from_millis(700)),
        ResponseExpectation::CapabilityGateway(NIP11_DOCUMENT),
    )?);
    negative_cases.push("total_deadline_timeout".to_owned());

    responses.push(run_exchange(
        address,
        endpoint,
        oracle,
        "malformed-protocol",
        OracleScenario::immediate(b"NOT-HTTP\r\nSensitive: remote\r\n\r\n".to_vec()),
        ResponseExpectation::CapabilityGateway(b"Sensitive: remote"),
    )?);
    negative_cases.push("malformed_protocol_to_local_502".to_owned());

    let cancellation = oracle.enqueue(OracleScenario::delayed(
        valid_response.clone(),
        Duration::from_millis(700),
    ))?;
    let cancelled_client = open_inbound_request(address)?;
    cancellation.wait_started()?;
    drop(cancelled_client);
    let cancelled_wire = cancellation.wait()?;
    if cancelled_wire.request_line.as_deref() != Some("GET /nip11 HTTP/1.1") {
        return Err(AcceptanceFailure::acceptance(
            "inbound_cancellation",
            "inbound cancellation did not reach the exact bounded upstream operation",
        ));
    }
    responses.push(HttpObservation {
        name: "inbound-client-cancellation".to_owned(),
        expected: "client-closed-no-partial-success".to_owned(),
        status: 0,
        body_bytes: 0,
        body_sha256: sha256_hex(&[]),
        elapsed_nanoseconds: cancelled_wire.elapsed_nanoseconds,
        upstream: cancelled_wire,
    });
    negative_cases.push("inbound_client_cancellation_closes_upstream".to_owned());

    let recovery = run_exchange(
        address,
        endpoint,
        oracle,
        "post-cancellation-recovery",
        OracleScenario::immediate(valid_response),
        ResponseExpectation::Success,
    )?;
    responses.push(recovery);
    context.stop_runner()?;
    Ok(first)
}

fn run_valid_once(
    context: &mut AcceptanceContext,
    cwd: &Path,
    descriptor: &Path,
    artifact_bundle: &str,
    oracle: &mut RawOracle,
    name: &str,
) -> Result<HttpObservation, AcceptanceFailure> {
    let ready = start_exact_runner(context, name, cwd, descriptor, artifact_bundle, None)?;
    let endpoint = oracle.endpoint(true, "localhost");
    let observation = run_exchange(
        ready_address(&ready)?,
        &endpoint,
        oracle,
        name,
        OracleScenario::immediate(raw_response(
            200,
            "OK",
            &[("Content-Type", "application/nostr+json")],
            NIP11_DOCUMENT,
        )),
        ResponseExpectation::Success,
    )?;
    context.stop_runner()?;
    Ok(observation)
}

fn exercise_plaintext(
    context: &mut AcceptanceContext,
    cwd: &Path,
    descriptor: &Path,
    artifact_bundle: &str,
    endpoint: &str,
    oracle: &mut RawOracle,
    responses: &mut Vec<HttpObservation>,
) -> Result<(), AcceptanceFailure> {
    let mut policy = DescriptorPolicy::trusted_tls(endpoint.to_owned());
    policy.trust = "webpki_roots";
    update_descriptor(descriptor, policy)?;
    let ready = start_exact_runner(
        context,
        "explicit-loopback-plaintext",
        cwd,
        descriptor,
        artifact_bundle,
        None,
    )?;
    responses.push(run_exchange(
        ready_address(&ready)?,
        endpoint,
        oracle,
        "explicit-loopback-plaintext",
        OracleScenario::immediate(raw_response(
            200,
            "OK",
            &[("Content-Type", "application/nostr+json")],
            NIP11_DOCUMENT,
        )),
        ResponseExpectation::Success,
    )?);
    context.stop_runner()
}

#[allow(clippy::too_many_arguments)]
fn exercise_destination_and_tls_failures(
    context: &mut AcceptanceContext,
    cwd: &Path,
    descriptor: &Path,
    artifact_bundle: &str,
    tls_endpoint: &str,
    fixture: &TlsFixture,
    tls_oracle: &mut RawOracle,
    expired_oracle: &mut RawOracle,
    responses: &mut Vec<HttpObservation>,
    no_connection: &mut Vec<NoConnectionObservation>,
    negative_cases: &mut Vec<String>,
) -> Result<(), AcceptanceFailure> {
    let mut public_policy = DescriptorPolicy::trusted_tls(tls_endpoint.to_owned());
    public_policy.address_policy = "public_only";
    update_descriptor(descriptor, public_policy)?;
    let ready = start_exact_runner(
        context,
        "public-policy-rejects-loopback",
        cwd,
        descriptor,
        artifact_bundle,
        None,
    )?;
    let response = request_inbound(ready_address(&ready)?)?;
    assert_gateway_response(&response, b"loopback must not connect", false)?;
    no_connection.push(
        tls_oracle.expect_none("public-policy-rejects-loopback", Duration::from_millis(200))?,
    );
    context.stop_runner()?;
    negative_cases.push("public_only_rejects_loopback_resolution".to_owned());

    update_descriptor(
        descriptor,
        DescriptorPolicy::trusted_tls(tls_endpoint.to_owned()),
    )?;
    let ready = start_exact_runner(
        context,
        "untrusted-root",
        cwd,
        descriptor,
        artifact_bundle,
        Some(&fixture.wrong_root_pem),
    )?;
    responses.push(run_exchange(
        ready_address(&ready)?,
        tls_endpoint,
        tls_oracle,
        "untrusted-root",
        OracleScenario::immediate(raw_response(
            200,
            "OK",
            &[("Content-Type", "application/nostr+json")],
            NIP11_DOCUMENT,
        )),
        ResponseExpectation::CapabilityGateway(NIP11_DOCUMENT),
    )?);
    context.stop_runner()?;
    negative_cases.push("untrusted_tls_chain_rejected".to_owned());

    let mismatch_endpoint = tls_oracle.endpoint(true, "127.0.0.1");
    update_descriptor(
        descriptor,
        DescriptorPolicy::trusted_tls(mismatch_endpoint.clone()),
    )?;
    let ready = start_exact_runner(
        context,
        "hostname-mismatch",
        cwd,
        descriptor,
        artifact_bundle,
        None,
    )?;
    responses.push(run_exchange(
        ready_address(&ready)?,
        &mismatch_endpoint,
        tls_oracle,
        "hostname-mismatch",
        OracleScenario::immediate(raw_response(
            200,
            "OK",
            &[("Content-Type", "application/nostr+json")],
            NIP11_DOCUMENT,
        )),
        ResponseExpectation::CapabilityGateway(NIP11_DOCUMENT),
    )?);
    context.stop_runner()?;
    negative_cases.push("tls_hostname_mismatch_rejected".to_owned());

    let expired_endpoint = expired_oracle.endpoint(true, "localhost");
    update_descriptor(
        descriptor,
        DescriptorPolicy::trusted_tls(expired_endpoint.clone()),
    )?;
    let ready = start_exact_runner(
        context,
        "expired-certificate",
        cwd,
        descriptor,
        artifact_bundle,
        None,
    )?;
    responses.push(run_exchange(
        ready_address(&ready)?,
        &expired_endpoint,
        expired_oracle,
        "expired-certificate",
        OracleScenario::immediate(raw_response(
            200,
            "OK",
            &[("Content-Type", "application/nostr+json")],
            NIP11_DOCUMENT,
        )),
        ResponseExpectation::CapabilityGateway(NIP11_DOCUMENT),
    )?);
    context.stop_runner()?;
    negative_cases.push("expired_tls_certificate_rejected".to_owned());

    let mut shutdown_policy = DescriptorPolicy::trusted_tls(tls_endpoint.to_owned());
    shutdown_policy.total_timeout_milliseconds = 5000;
    shutdown_policy.cleanup_timeout_milliseconds = 1500;
    update_descriptor(descriptor, shutdown_policy)?;
    let ready = start_exact_runner(
        context,
        "shutdown-during-outbound",
        cwd,
        descriptor,
        artifact_bundle,
        None,
    )?;
    let address = ready_address(&ready)?;
    let ticket = tls_oracle.enqueue(OracleScenario::delayed(
        raw_response(
            200,
            "OK",
            &[("Content-Type", "application/nostr+json")],
            NIP11_DOCUMENT,
        ),
        Duration::from_secs(2),
    ))?;
    let request_thread = thread::Builder::new()
        .name("outbound-http-shutdown-probe".to_owned())
        .spawn(move || http_probe::request(address, "GET", "/relay-info", &[], &[]))
        .map_err(|error| AcceptanceFailure::infrastructure("shutdown_probe", error))?;
    ticket.wait_started()?;
    context.stop_runner()?;
    let wire = ticket.wait()?;
    let _ = request_thread.join().map_err(|_| {
        AcceptanceFailure::infrastructure("shutdown_probe", "shutdown probe panicked")
    })?;
    if wire.request_line.as_deref() != Some("GET /nip11 HTTP/1.1") {
        return Err(AcceptanceFailure::acceptance(
            "shutdown_upstream",
            "shutdown case did not observe the exact outbound request",
        ));
    }
    responses.push(HttpObservation {
        name: "runner-shutdown-cancellation".to_owned(),
        expected: "shutdown-closed-no-partial-success".to_owned(),
        status: 0,
        body_bytes: 0,
        body_sha256: sha256_hex(&[]),
        elapsed_nanoseconds: wire.elapsed_nanoseconds,
        upstream: wire,
    });
    negative_cases.push("runner_shutdown_cancels_outbound_and_cleans_resources".to_owned());
    Ok(())
}

fn exercise_startup_failure(
    context: &mut AcceptanceContext,
    cwd: &Path,
    descriptor: &Path,
    endpoint: &str,
) -> Result<u64, AcceptanceFailure> {
    update_descriptor(
        descriptor,
        DescriptorPolicy::trusted_tls(endpoint.to_owned()),
    )?;
    let valid = std::mem::replace(
        &mut context.root_pem,
        "-----BEGIN CERTIFICATE-----\nnot-base64\n-----END CERTIFICATE-----\n".to_owned(),
    );
    let result = context.invoke(
        "invalid-root-no-ready",
        vec![
            "serve".to_owned(),
            "--deployment".to_owned(),
            descriptor.display().to_string(),
        ],
        cwd,
        ExpectedCommand::RuntimeFailure,
    );
    context.root_pem = valid;
    result?;
    Ok(1)
}

fn start_exact_runner(
    context: &mut AcceptanceContext,
    name: &str,
    cwd: &Path,
    descriptor: &Path,
    artifact_bundle: &str,
    root_override: Option<&str>,
) -> Result<ReadyObservation, AcceptanceFailure> {
    let arguments = vec![
        "serve".to_owned(),
        "--deployment".to_owned(),
        descriptor.display().to_string(),
    ];
    let ready = match root_override {
        Some(root) => context.start_runner_with_root(name, arguments, cwd, root)?,
        None => context.start_runner(name, arguments, cwd)?,
    };
    if ready.artifact_digest != artifact_bundle
        || ready.target != "serve"
        || ready.runner != "http"
        || ready.configured_listener != "127.0.0.1:0"
        || ready.grants.get("streams").map(String::as_str) != Some("byte-stream")
        || ready.grants.get("relay").map(String::as_str) != Some("http-client")
    {
        return Err(AcceptanceFailure::acceptance(
            "ready_identity",
            "ready event disagrees with the exact artifact, target, listener, or grants",
        ));
    }
    Ok(ready)
}

fn run_exchange(
    inbound: SocketAddr,
    endpoint: &str,
    oracle: &RawOracle,
    name: &str,
    scenario: OracleScenario,
    expected: ResponseExpectation<'_>,
) -> Result<HttpObservation, AcceptanceFailure> {
    let ticket = oracle.enqueue(scenario)?;
    let response = request_inbound(inbound)?;
    let wire = ticket.wait()?;
    match expected {
        ResponseExpectation::Success => {
            if response.status != 200 || response.body != NIP11_DOCUMENT {
                return Err(AcceptanceFailure::acceptance(
                    "nip11_response",
                    format!("{name} did not preserve the exact bounded NIP-11 document"),
                ));
            }
            if response.headers.get("content-type").map(String::as_str)
                != Some("application/nostr+json")
            {
                return Err(AcceptanceFailure::acceptance(
                    "nip11_content_type",
                    format!("{name} did not return the exact local NIP-11 media type"),
                ));
            }
            assert_exact_wire(endpoint, &wire)?;
        }
        ResponseExpectation::GraphGateway(marker) => {
            assert_gateway_response(&response, marker, true)?;
            assert_exact_wire(endpoint, &wire)?;
        }
        ResponseExpectation::CapabilityGateway(marker) => {
            assert_gateway_response(&response, marker, false)?;
            if wire.request_line.is_some() {
                assert_exact_wire(endpoint, &wire)?;
            }
        }
    }
    Ok(HttpObservation {
        name: name.to_owned(),
        expected: match expected {
            ResponseExpectation::Success => "status-200-exact-document",
            ResponseExpectation::GraphGateway(_) => "graph-local-502",
            ResponseExpectation::CapabilityGateway(_) => "redacted-capability-502",
        }
        .to_owned(),
        status: response.status,
        body_bytes: response.body.len() as u64,
        body_sha256: sha256_hex(&response.body),
        elapsed_nanoseconds: response.elapsed_nanoseconds,
        upstream: wire,
    })
}

fn request_inbound(address: SocketAddr) -> Result<http_probe::HttpResponse, AcceptanceFailure> {
    http_probe::request(address, "GET", "/relay-info", &[], &[])
        .map_err(|error| AcceptanceFailure::infrastructure("inbound_request", error))
}

fn open_inbound_request(address: SocketAddr) -> Result<TcpStream, AcceptanceFailure> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| AcceptanceFailure::infrastructure("inbound_cancel_connect", error))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| AcceptanceFailure::infrastructure("inbound_cancel_timeout", error))?;
    write!(
        stream,
        "GET /relay-info HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    )
    .and_then(|()| stream.flush())
    .map_err(|error| AcceptanceFailure::infrastructure("inbound_cancel_write", error))?;
    Ok(stream)
}

fn assert_gateway_response(
    response: &http_probe::HttpResponse,
    remote_marker: &[u8],
    graph_selected: bool,
) -> Result<(), AcceptanceFailure> {
    let allowed = if graph_selected {
        response.body.as_slice() == BAD_GATEWAY_BODY
    } else {
        matches!(
            response.body.as_slice(),
            b"request could not be completed"
                | b"request resource limit reached"
                | b"request cancelled"
        )
    };
    if response.status != 502
        || !allowed
        || (!remote_marker.is_empty()
            && response
                .body
                .windows(remote_marker.len())
                .any(|window| window == remote_marker))
    {
        return Err(AcceptanceFailure::acceptance(
            "gateway_response",
            format!(
                "application did not return the deterministic redacted local 502 response (status={}, body={:?}, graph_selected={graph_selected})",
                response.status,
                String::from_utf8_lossy(&response.body)
            ),
        ));
    }
    Ok(())
}

fn assert_exact_wire(endpoint: &str, wire: &WireObservation) -> Result<(), AcceptanceFailure> {
    let authority = endpoint
        .split_once("://")
        .and_then(|(_, suffix)| suffix.split('/').next())
        .ok_or_else(|| {
            AcceptanceFailure::acceptance("wire_endpoint", "oracle endpoint malformed")
        })?;
    if !wire.tls_established && endpoint.starts_with("https://")
        || wire.request_line.as_deref() != Some("GET /nip11 HTTP/1.1")
        || wire.headers.get("host").map(String::as_str) != Some(authority)
        || wire.headers.get("accept").map(String::as_str) != Some("application/nostr+json")
        || wire.headers.get("accept-encoding").map(String::as_str) != Some("identity")
        || wire.headers.get("connection").map(String::as_str) != Some("close")
        || wire.headers.len() != 4
        || wire.headers.keys().any(|name| {
            matches!(
                name.as_str(),
                "authorization"
                    | "cookie"
                    | "proxy-authorization"
                    | "content-length"
                    | "transfer-encoding"
                    | "upgrade"
            )
        })
    {
        return Err(AcceptanceFailure::acceptance(
            "wire_request",
            "outbound wire request was not the exact bounded GET/Host/Accept/identity/close shape",
        ));
    }
    Ok(())
}

fn raw_response(status: u16, reason: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    raw_response_owned_headers(
        status,
        reason,
        headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        body,
    )
}

fn raw_response_owned_headers(
    status: u16,
    reason: &str,
    headers: Vec<(String, String)>,
    body: &[u8],
) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {status} {reason}\r\n").into_bytes();
    for (name, value) in headers {
        response.extend_from_slice(name.as_bytes());
        response.extend_from_slice(b": ");
        response.extend_from_slice(value.as_bytes());
        response.extend_from_slice(b"\r\n");
    }
    response.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    response.extend_from_slice(b"Connection: close\r\n\r\n");
    response.extend_from_slice(body);
    response
}

fn network_resources(responses: &[HttpObservation]) -> NetworkResources {
    let mut resources = NetworkResources {
        connections: 0,
        request_bytes: 0,
        response_bytes: 0,
        maximum_observed_request_bytes: 0,
        maximum_observed_response_bytes: 0,
    };
    for response in responses {
        let wire = &response.upstream;
        resources.connections = resources.connections.saturating_add(1);
        resources.request_bytes = resources.request_bytes.saturating_add(wire.request_bytes);
        resources.response_bytes = resources.response_bytes.saturating_add(wire.response_bytes);
        resources.maximum_observed_request_bytes = resources
            .maximum_observed_request_bytes
            .max(wire.request_bytes);
        resources.maximum_observed_response_bytes = resources
            .maximum_observed_response_bytes
            .max(wire.response_bytes);
    }
    resources
}

fn require_template_discovery(records: &[CompactRecord]) -> Result<(), AcceptanceFailure> {
    let template = records.iter().find(|record| {
        record.operation == "template"
            && record
                .fields
                .iter()
                .any(|field| field.name == "name" && field.value == "nostr-relay-info")
    });
    let operation = records.iter().find(|record| {
        record.operation == "operation"
            && record
                .fields
                .iter()
                .any(|field| field.name == "name" && field.value == "new")
    });
    let usage = operation
        .and_then(|record| record.fields.iter().find(|field| field.name == "usage"))
        .map(|field| field.value.as_str());
    if template.is_none()
        || usage.is_none_or(|usage| {
            !usage.contains("nostr-relay-info") || !usage.contains("--relay-url URL")
        })
    {
        return Err(AcceptanceFailure::acceptance(
            "template_discovery",
            "candidate discovery omitted nostr-relay-info or --relay-url",
        ));
    }
    Ok(())
}

fn require_next_actions(records: &[CompactRecord]) -> Result<(), AcceptanceFailure> {
    let actions = records
        .iter()
        .filter(|record| record.operation == "next")
        .map(|record| {
            Ok((
                parse_u64(required_field(record, "order")?, "next order")?,
                required_field(record, "kind")?.to_owned(),
            ))
        })
        .collect::<Result<Vec<_>, AcceptanceFailure>>()?;
    let expected = vec![
        (1, "status".to_owned()),
        (2, "check".to_owned()),
        (3, "build".to_owned()),
        (4, "serve".to_owned()),
    ];
    if actions != expected {
        return Err(AcceptanceFailure::acceptance(
            "next_actions",
            "new Nostr recipe output does not expose the exact ordered lifecycle",
        ));
    }
    Ok(())
}

fn compiler_observation(
    records: &[CompactRecord],
) -> Result<CompilerObservation, AcceptanceFailure> {
    let record = required_record(records, "compilation")?;
    Ok(CompilerObservation {
        cache: required_field(record, "cache")?.to_owned(),
        compiled: parse_u64(required_field(record, "compiled")?, "compiled units")?,
        reused: parse_u64(required_field(record, "reused")?, "reused units")?,
        removed: parse_u64(required_field(record, "removed")?, "removed units")?,
    })
}

fn compact_records(label: &str, bytes: &[u8]) -> Result<Vec<CompactRecord>, AcceptanceFailure> {
    parse_records(label, bytes).map_err(|diagnostics| {
        let codes = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>()
            .join(",");
        AcceptanceFailure::acceptance(
            "compact_output",
            format!("{label} output is not strict compact records ({codes})"),
        )
    })
}

fn required_record<'a>(
    records: &'a [CompactRecord],
    operation: &str,
) -> Result<&'a CompactRecord, AcceptanceFailure> {
    records
        .iter()
        .find(|record| record.operation == operation)
        .ok_or_else(|| {
            AcceptanceFailure::acceptance(
                "compact_record",
                format!("compact output omitted '{operation}'"),
            )
        })
}

fn required_field<'a>(record: &'a CompactRecord, name: &str) -> Result<&'a str, AcceptanceFailure> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
        .ok_or_else(|| {
            AcceptanceFailure::acceptance(
                "compact_field",
                format!("compact '{}' record omitted '{name}'", record.operation),
            )
        })
}

fn require_field(
    records: &[CompactRecord],
    operation: &str,
    field: &str,
    expected: &str,
) -> Result<(), AcceptanceFailure> {
    let actual = required_field(required_record(records, operation)?, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(AcceptanceFailure::acceptance(
            "exact_output",
            format!("{operation}.{field} is '{actual}', expected '{expected}'"),
        ))
    }
}

fn parse_u64(value: &str, label: &str) -> Result<u64, AcceptanceFailure> {
    value.parse::<u64>().map_err(|error| {
        AcceptanceFailure::infrastructure(
            "numeric_output",
            format!("{label} is not an unsigned integer: {error}"),
        )
    })
}

fn parse_ready(line: &[u8]) -> Result<ReadyObservation, AcceptanceFailure> {
    let value: Value = serde_json::from_slice(line)
        .map_err(|error| AcceptanceFailure::infrastructure("ready_json", error))?;
    if value.get("event").and_then(Value::as_str) != Some("ready")
        || value.get("ok").and_then(Value::as_bool) != Some(true)
    {
        return Err(AcceptanceFailure::acceptance(
            "ready_event",
            "first resident event is not successful readiness",
        ));
    }
    let deployment = value.get("deployment").ok_or_else(|| {
        AcceptanceFailure::acceptance("ready_deployment", "ready event omitted deployment")
    })?;
    let grants = deployment
        .get("grants")
        .and_then(Value::as_object)
        .ok_or_else(|| AcceptanceFailure::acceptance("ready_grants", "ready omitted grants"))?
        .iter()
        .map(|(name, value)| {
            value
                .as_str()
                .map(|kind| (name.clone(), kind.to_owned()))
                .ok_or_else(|| {
                    AcceptanceFailure::acceptance("ready_grants", "ready grant is not text")
                })
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(ReadyObservation {
        artifact_digest: json_text(deployment, "artifact_digest", "ready artifact")?,
        target: json_text(deployment, "target", "ready target")?,
        runner: json_text(deployment, "runner", "ready runner")?,
        configured_listener: json_text(deployment, "listen", "ready listener")?,
        local_address: json_text(&value, "local_address", "bound listener")?,
        grants,
        elapsed_nanoseconds: 0,
    })
}

fn parse_stopped(bytes: &[u8]) -> Result<StoppedObservation, AcceptanceFailure> {
    let line = bytes
        .split(|byte| *byte == b'\n')
        .find(|line| {
            serde_json::from_slice::<Value>(line)
                .ok()
                .and_then(|value| {
                    value
                        .get("event")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .as_deref()
                == Some("stopped")
        })
        .ok_or_else(|| {
            AcceptanceFailure::acceptance("stopped_event", "runner omitted stopped event")
        })?;
    let value: Value = serde_json::from_slice(line)
        .map_err(|error| AcceptanceFailure::infrastructure("stopped_json", error))?;
    let receipt = value.get("receipt").ok_or_else(|| {
        AcceptanceFailure::acceptance("stopped_receipt", "stopped event omitted receipt")
    })?;
    let shutdown = receipt.get("shutdown").ok_or_else(|| {
        AcceptanceFailure::acceptance("stopped_shutdown", "stopped receipt omitted shutdown")
    })?;
    Ok(StoppedObservation {
        admission_stopped: shutdown
            .get("admission_stopped")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        remaining_tasks: shutdown
            .get("remaining_tasks")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX),
        cleanup_failures: shutdown
            .get("cleanup_failures")
            .and_then(Value::as_array)
            .map(|values| values.len() as u64)
            .unwrap_or(u64::MAX),
        elapsed_nanoseconds: shutdown
            .get("elapsed_nanoseconds")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX),
    })
}

fn ready_address(ready: &ReadyObservation) -> Result<SocketAddr, AcceptanceFailure> {
    let address = ready
        .local_address
        .parse::<SocketAddr>()
        .map_err(|error| AcceptanceFailure::infrastructure("ready_address", error))?;
    if address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err(AcceptanceFailure::acceptance(
            "ready_address",
            "application listener is not exact IPv4 loopback",
        ));
    }
    Ok(address)
}

fn json_text(value: &Value, field: &str, label: &str) -> Result<String, AcceptanceFailure> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            AcceptanceFailure::acceptance("runtime_event_field", format!("{label} is absent"))
        })
}

fn first_line(path: &Path) -> Result<Option<Vec<u8>>, AcceptanceFailure> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(AcceptanceFailure::infrastructure("runner_log", error)),
    };
    if bytes.len() as u64 > MAXIMUM_COMMAND_OUTPUT_BYTES {
        return Err(AcceptanceFailure::acceptance(
            "runner_log_limit",
            "resident stdout exceeded the acceptance bound",
        ));
    }
    Ok(bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|end| bytes[..end].to_vec()))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    (!needle.is_empty() && needle.len() <= haystack.len())
        .then(|| {
            haystack
                .windows(needle.len())
                .position(|window| window == needle)
        })
        .flatten()
}

fn parse_options(arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let mut binary = None;
    let mut evidence_root = None;
    let mut machine = false;
    let mut arguments = arguments;
    while let Some(argument) = crate::next_utf8(&mut arguments, "outbound HTTP option")? {
        match argument.as_str() {
            "--binary" => {
                if binary.is_some() {
                    return Err(DevError::usage("duplicate --binary option"));
                }
                let value = crate::next_utf8(&mut arguments, "--binary path")?
                    .ok_or_else(|| DevError::usage("--binary requires a path"))?;
                binary = Some(PathBuf::from(value));
            }
            "--evidence-root" => {
                if evidence_root.is_some() {
                    return Err(DevError::usage("duplicate --evidence-root option"));
                }
                let value = crate::next_utf8(&mut arguments, "--evidence-root path")?
                    .ok_or_else(|| DevError::usage("--evidence-root requires a path"))?;
                evidence_root = Some(PathBuf::from(value));
            }
            "--machine" => {
                if machine {
                    return Err(DevError::usage("duplicate --machine option"));
                }
                machine = true;
            }
            other => return Err(DevError::usage(format!("unknown option '{other}'"))),
        }
    }
    Ok(Options {
        binary: binary.unwrap_or_else(|| PathBuf::from("target/release/lkjscript")),
        evidence_root,
        machine,
    })
}

fn resolve_candidate(checkout_root: Option<&Path>, binary: &Path) -> Result<PathBuf, DevError> {
    let path = if binary.is_absolute() {
        binary.to_path_buf()
    } else {
        let repository = checkout_root.ok_or_else(|| {
            DevError::usage(
                "--binary must be absolute when --evidence-root selects transferred mode",
            )
        })?;
        repository.join(binary)
    };
    resolve_regular_executable(&path, "outbound HTTP candidate")
}

fn current_verifier() -> Result<PathBuf, DevError> {
    let path = std::env::current_exe()
        .map_err(|error| DevError::infrastructure(format!("resolve verifier: {error}")))?;
    resolve_regular_executable(&path, "outbound HTTP verifier")
}

fn resolve_regular_executable(path: &Path, label: &str) -> Result<PathBuf, DevError> {
    if !path.is_absolute() || has_noncanonical_component(path) {
        return Err(DevError::usage(format!(
            "{label} path '{}' must be absolute and lexically canonical",
            path.display()
        )));
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect {label} '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAXIMUM_EXECUTABLE_BYTES
    {
        return Err(DevError::usage(format!(
            "{label} '{}' must be a bounded regular non-symlink file",
            path.display()
        )));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::usage(format!(
            "{label} '{}' is not executable",
            path.display()
        )));
    }
    let canonical = path.canonicalize().map_err(|error| {
        DevError::infrastructure(format!("resolve {label} '{}': {error}", path.display()))
    })?;
    if canonical != path {
        return Err(DevError::usage(format!(
            "{label} path '{}' contains a symlink or noncanonical component",
            path.display()
        )));
    }
    Ok(canonical)
}

fn executable_observation(path: &Path, label: &str) -> Result<ExecutableObservation, DevError> {
    let path = resolve_regular_executable(path, label)?;
    let file = evidence::proof(&path, path.display().to_string())?;
    let byte_length = file
        .bytes
        .ok_or_else(|| DevError::infrastructure(format!("{label} proof omitted byte length")))?;
    let mode = file
        .mode
        .ok_or_else(|| DevError::infrastructure(format!("{label} proof omitted file mode")))?;
    let verification_digest = file
        .digest
        .clone()
        .ok_or_else(|| DevError::infrastructure(format!("{label} proof omitted digest")))?;
    Ok(ExecutableObservation {
        file,
        byte_length,
        mode,
        sha256: sha256_file(&path, MAXIMUM_EXECUTABLE_BYTES)?,
        verification_digest,
    })
}

fn copy_binary(source: &Path, destination: &Path) -> Result<(), DevError> {
    let input = File::open(source)?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o755);
    let mut output = options.open(destination)?;
    let copied = io::copy(
        &mut input.take(MAXIMUM_EXECUTABLE_BYTES.saturating_add(1)),
        &mut output,
    )?;
    if copied > MAXIMUM_EXECUTABLE_BYTES {
        return Err(DevError::infrastructure(
            "outbound candidate exceeded copy bound",
        ));
    }
    output.sync_all()?;
    #[cfg(unix)]
    fs::set_permissions(destination, fs::Permissions::from_mode(0o755))?;
    File::open(
        destination
            .parent()
            .ok_or_else(|| DevError::infrastructure("copied candidate has no parent"))?,
    )?
    .sync_all()?;
    Ok(())
}

fn create_explicit_evidence_root(requested: &Path) -> Result<PathBuf, DevError> {
    if !requested.is_absolute() || has_noncanonical_component(requested) {
        return Err(DevError::usage(format!(
            "evidence root '{}' must be absolute and lexically canonical",
            requested.display()
        )));
    }
    if fs::symlink_metadata(requested).is_ok() {
        return Err(DevError::usage(format!(
            "evidence root '{}' already exists",
            requested.display()
        )));
    }
    let parent = requested.parent().ok_or_else(|| {
        DevError::usage(format!(
            "evidence root '{}' has no parent directory",
            requested.display()
        ))
    })?;
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        DevError::usage(format!(
            "inspect evidence-root parent '{}': {error}",
            parent.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::usage(
            "evidence-root parent must be a regular non-symlink directory",
        ));
    }
    let canonical_parent = parent.canonicalize()?;
    if canonical_parent != parent {
        return Err(DevError::usage(
            "evidence-root parent contains a noncanonical component",
        ));
    }
    let name = requested
        .file_name()
        .ok_or_else(|| DevError::usage("evidence root has no private directory name"))?;
    let root = canonical_parent.join(name);
    fs::create_dir(&root)?;
    #[cfg(unix)]
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    File::open(&canonical_parent)?.sync_all()?;
    let canonical_root = root.canonicalize()?;
    if canonical_root != root {
        let _ = fs::remove_dir(&root);
        return Err(DevError::infrastructure(
            "created evidence root escaped its canonical parent",
        ));
    }
    Ok(canonical_root)
}

fn new_evidence_directory(repository: &Path) -> Result<PathBuf, DevError> {
    let ordinal = RUN_ORDINAL.fetch_add(1, Ordering::Relaxed);
    let parent = repository.join(".artifacts/lkjscript-dev/outbound-http");
    fs::create_dir_all(&parent)?;
    let metadata = fs::symlink_metadata(&parent)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::infrastructure(
            "outbound HTTP evidence parent is not a regular directory",
        ));
    }
    let directory = parent.join(format!(
        "{}-{}-{ordinal}",
        unix_nanoseconds()?,
        std::process::id()
    ));
    fs::create_dir(&directory)?;
    #[cfg(unix)]
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    Ok(directory)
}

fn repository_root() -> Result<PathBuf, DevError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| DevError::infrastructure("lkjscript-dev escaped its workspace"))?
        .canonicalize()
        .map_err(|error| DevError::infrastructure(format!("resolve repository root: {error}")))
}

fn has_noncanonical_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

fn isolated_environment(root_pem: Option<&str>) -> BTreeMap<String, String> {
    let mut environment = BTreeMap::from([("LANG".to_owned(), "C".to_owned())]);
    if let Some(root_pem) = root_pem {
        environment.insert(ROOT_ENVIRONMENT.to_owned(), root_pem.to_owned());
    }
    environment
}

fn safe_name(value: &str) -> Result<String, AcceptanceFailure> {
    let normalized = value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || byte == b'-' {
                char::from(byte)
            } else {
                '-'
            }
        })
        .collect::<String>();
    if normalized.is_empty() || normalized.bytes().all(|byte| byte == b'-') {
        Err(AcceptanceFailure::acceptance(
            "evidence_name",
            "evidence name is not a safe file component",
        ))
    } else {
        Ok(normalized)
    }
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock: {error}")))
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn sha256_hex(bytes: &[u8]) -> String {
    lowercase_hex(&Sha256::digest(bytes))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        output.push(char::from(HEX[usize::from(*byte >> 4)]));
        output.push(char::from(HEX[usize::from(*byte & 0x0f)]));
    }
    output
}

fn sha256_file(path: &Path, maximum_bytes: u64) -> Result<String, DevError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > maximum_bytes {
        return Err(DevError::infrastructure(format!(
            "SHA-256 input '{}' is unsafe or excessive",
            path.display()
        )));
    }
    let mut input = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut observed = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        observed = observed
            .checked_add(read as u64)
            .ok_or_else(|| DevError::infrastructure("SHA-256 byte length overflow"))?;
        hasher.update(&buffer[..read]);
    }
    if observed != metadata.len() {
        return Err(DevError::infrastructure(
            "SHA-256 input changed while reading",
        ));
    }
    Ok(lowercase_hex(&hasher.finalize()))
}

fn print_summary(
    options: &Options,
    receipt: &AcceptanceReceipt,
    published: &PublishedEvidence,
    receipt_sha256: &str,
) -> Result<(), DevError> {
    if options.machine {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "status": receipt.status,
                "schema": receipt.schema,
                "workflow": receipt.workflow,
                "receipt": published.path,
                "receipt_bytes": published.bytes,
                "receipt_digest": published.digest,
                "receipt_sha256": receipt_sha256,
                "verifier_sha256": receipt.verifier.sha256,
                "candidate_sha256": receipt.candidate.sha256,
                "elapsed_nanoseconds": receipt.elapsed_nanoseconds,
                "commands": receipt.commands.len(),
                "runners": receipt.runners.len(),
                "requests": receipt.result.as_ref().map_or(0, |result| result.responses.len()),
            }))?
        );
    } else {
        println!(
            "outbound HTTP application: status={:?} commands={} runners={} receipt={} digest={}",
            receipt.status,
            receipt.commands.len(),
            receipt.runners.len(),
            published.path.display(),
            published.digest
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_tls_fixture_is_stable_and_hostname_bound() {
        let first = TlsFixture::generate().expect("first TLS fixture");
        let second = TlsFixture::generate().expect("second TLS fixture");
        assert_eq!(first.root_pem, second.root_pem);
        assert_eq!(
            first.observation.root_der_sha256,
            second.observation.root_der_sha256
        );
        assert_eq!(
            first.observation.leaf_der_sha256,
            second.observation.leaf_der_sha256
        );
        assert_ne!(
            first.observation.leaf_der_sha256,
            first.observation.expired_leaf_der_sha256
        );
        assert_eq!(first.observation.hostname, "localhost");
    }

    #[test]
    fn raw_oracle_request_parser_is_independent_and_strict() {
        let (line, headers) = parse_raw_request(
            b"GET /nip11 HTTP/1.1\r\nHost: localhost:7447\r\nAccept: application/nostr+json\r\n\r\n",
        )
        .expect("raw request");
        assert_eq!(line, "GET /nip11 HTTP/1.1");
        assert_eq!(
            headers.get("accept").map(String::as_str),
            Some("application/nostr+json")
        );
        assert!(parse_raw_request(b"GET / HTTP/1.1\r\nX: 1\r\nX: 2\r\n\r\n").is_err());
        assert!(parse_raw_request(b"GET / HTTP/1.1\r\n\r\nbody").is_err());
    }

    #[test]
    fn outbound_options_are_closed() {
        assert!(parse_options([OsString::from("--machine")].into_iter()).is_ok());
        assert!(parse_options([OsString::from("--unknown")].into_iter()).is_err());
        assert!(
            parse_options([OsString::from("--binary"), OsString::from("one")].into_iter()).is_ok()
        );
    }
}
