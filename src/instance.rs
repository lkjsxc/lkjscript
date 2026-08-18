//! Durable typed application instances and the narrow activation host boundary.

use crate::application::{
    self, ApplicationDigest, ApplicationValue, HostOutcomeKind, InvocationProfile, StatefulCommand,
    StatefulCommandKind, StatefulTransition,
};
use crate::codec::{CodecError, Reader, Writer};
use crate::error::{ErrorCode, LkError, Result};
use crate::schema::ByteString;
use fs2::FileExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

pub const INSTANCE_CONTRACT_VERSION: u16 = 1;
pub const INSTANCE_FORMAT_VERSION: u16 = 1;
pub const MAXIMUM_INSTANCE_STATE_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_INSTANCE_EVENT_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_INSTANCE_HISTORY_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_INSTANCE_TRANSITIONS: usize = 10_000;
pub const MAXIMUM_INSTANCE_REPLAY_WORK: usize = 10_000;
pub const MAXIMUM_INSTANCE_HISTORY_PAGE: usize = 256;
pub const MAXIMUM_EVENT_KEY_BYTES: usize = 96;
pub const MAXIMUM_GRANT_NAME_BYTES: usize = 64;
pub const MAXIMUM_HOST_EVIDENCE_BYTES: usize = 64 * 1024;
pub const MAXIMUM_INSTANCE_PATH_BYTES: usize = 4_096;
const MAXIMUM_INSTANCE_RECORD_BYTES: usize = 4 * 1024 * 1024;

const RECORD_MAGIC: [u8; 8] = *b"LKJINS\0\x01";
const OUTCOME_MAGIC: [u8; 8] = *b"LKJOUT\0\x01";
const ATTEMPT_MAGIC: [u8; 8] = *b"LKJATT\0\x01";
const HEAD_MAGIC: [u8; 8] = *b"LKJIHEAD";
const RECORD_DOMAIN: &str = "lkjscript.instance-record.v1";
const OUTCOME_DOMAIN: &str = "lkjscript.instance-host-outcome.v1";
const ATTEMPT_DOMAIN: &str = "lkjscript.instance-host-attempt.v1";
const HEAD_DOMAIN: &str = "lkjscript.instance-head.v1";
const STATE_DOMAIN: &str = "lkjscript.instance-state.v1";
const GRANT_DOMAIN: &str = "lkjscript.activation-grant.v1";
const COMMAND_DOMAIN: &str = "lkjscript.instance-command.v1";
const LOCK_FILE: &str = "lkjscript.instance.lock";
const APPLICATION_FILE: &str = "application.lkja";
const HEAD_FILE: &str = "HEAD";
static TEMPORARY_SERIAL: AtomicU64 = AtomicU64::new(1);

macro_rules! exact_bytes {
    ($name:ident, $bytes:expr, $label:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; $bytes]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; $bytes]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(self) -> [u8; $bytes] {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&hex(&self.0))
            }
        }

        impl FromStr for $name {
            type Err = &'static str;

            fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
                decode_hex::<$bytes>(value).map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                value
                    .parse()
                    .map_err(|_| de::Error::custom(concat!($label, " is not canonical hex")))
            }
        }
    };
}

exact_bytes!(InstanceId, 16, "instance ID");
exact_bytes!(StateDigest, 32, "state digest");
exact_bytes!(InstanceRecordDigest, 32, "instance record digest");
exact_bytes!(ActivationGrantDigest, 32, "activation grant digest");
exact_bytes!(CommandId, 32, "command ID");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceMode {
    ValidateOnly,
    Commit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostExecutorKind {
    Production,
    DeterministicFake,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstancePolicy {
    pub maximum_state_bytes: u64,
    pub maximum_event_bytes: u64,
    pub maximum_history_bytes: u64,
    pub maximum_transitions: u64,
    pub maximum_replay_work: u64,
}

impl Default for InstancePolicy {
    fn default() -> Self {
        Self {
            maximum_state_bytes: MAXIMUM_INSTANCE_STATE_BYTES as u64,
            maximum_event_bytes: MAXIMUM_INSTANCE_EVENT_BYTES as u64,
            maximum_history_bytes: MAXIMUM_INSTANCE_HISTORY_BYTES as u64,
            maximum_transitions: MAXIMUM_INSTANCE_TRANSITIONS as u64,
            maximum_replay_work: MAXIMUM_INSTANCE_REPLAY_WORK as u64,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationGrant {
    pub version: u16,
    pub name: String,
    pub instance: InstanceId,
    pub executor: HostExecutorKind,
    pub source_directory: String,
    pub slot: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceCreateRequest {
    pub version: u16,
    pub mode: InstanceMode,
    pub instance: InstanceId,
    pub initial_state: ApplicationValue,
    pub grant: ActivationGrant,
    #[serde(default)]
    pub policy: InstancePolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceEventRequest {
    pub version: u16,
    pub mode: InstanceMode,
    pub instance: InstanceId,
    pub base_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_key: Option<String>,
    pub event: ApplicationValue,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceResumeRequest {
    pub version: u16,
    pub mode: InstanceMode,
    pub instance: InstanceId,
    pub base_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceHostRequest {
    pub version: u16,
    pub instance: InstanceId,
    pub command: CommandId,
    pub grant: ActivationGrant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_application: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceFakeHostRequest {
    pub version: u16,
    pub instance: InstanceId,
    pub command: CommandId,
    pub grant: ActivationGrant,
    pub outcome: HostOutcomeKind,
    pub evidence: ByteString,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceDeleteRequest {
    pub version: u16,
    pub instance: InstanceId,
    pub base_revision: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PendingCommand {
    pub id: CommandId,
    pub kind: StatefulCommandKind,
    pub application: ApplicationDigest,
    pub grant: ActivationGrantDigest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceTransitionStatus {
    Completed,
    Suspended,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceTransitionReceipt {
    pub contract_version: u16,
    pub instance: InstanceId,
    pub application: ApplicationDigest,
    pub base_revision: u64,
    pub next_revision: u64,
    pub state_digest: StateDigest,
    pub response: ByteString,
    pub status: InstanceTransitionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<PendingCommand>,
    pub published: bool,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceCreateReceipt {
    pub contract_version: u16,
    pub instance: InstanceId,
    pub application: ApplicationDigest,
    pub revision: u64,
    pub state_digest: StateDigest,
    pub grant: ActivationGrantDigest,
    pub published: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostExecutionReceipt {
    pub contract_version: u16,
    pub instance: InstanceId,
    pub command: CommandId,
    pub outcome: HostOutcomeKind,
    pub evidence: ByteString,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceInspection {
    pub contract_version: u16,
    pub format_version: u16,
    pub instance: InstanceId,
    pub application: ApplicationDigest,
    pub revision: u64,
    pub record_digest: InstanceRecordDigest,
    pub state_digest: StateDigest,
    pub state: ApplicationValue,
    pub response: ByteString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_command: Option<PendingCommand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_outcome: Option<HostOutcomeKind>,
    pub host_attempted: bool,
    pub grant_name: String,
    pub grant: ActivationGrantDigest,
    pub policy: InstancePolicy,
    pub history_records: u64,
    pub history_bytes: u64,
    pub deleted: bool,
    pub legal_actions: Vec<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceHistoryItem {
    pub revision: u64,
    pub record_digest: InstanceRecordDigest,
    pub state_digest: StateDigest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_key: Option<String>,
    pub status: InstanceTransitionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<PendingCommand>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceHistoryPage {
    pub contract_version: u16,
    pub instance: InstanceId,
    pub start_revision: u64,
    pub next_revision: u64,
    pub complete: bool,
    pub items: Vec<InstanceHistoryItem>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum TransitionInput {
    External {
        base_revision: u64,
        event: ApplicationValue,
    },
    Host {
        base_revision: u64,
        command: CommandId,
        outcome: HostOutcomeKind,
        evidence: ByteString,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct InstanceRecord {
    instance: InstanceId,
    application: ApplicationDigest,
    revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    prior: Option<InstanceRecordDigest>,
    state: ApplicationValue,
    state_digest: StateDigest,
    response: ByteString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    event_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    input: Option<TransitionInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<PendingCommand>,
    grant_name: String,
    grant: ActivationGrantDigest,
    policy: InstancePolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct InstanceHead {
    instance: InstanceId,
    revision: u64,
    record: InstanceRecordDigest,
    deleted: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HostOutcomeRecord {
    instance: InstanceId,
    command: CommandId,
    outcome: HostOutcomeKind,
    evidence: ByteString,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HostAttemptRecord {
    instance: InstanceId,
    command: CommandId,
}

struct LoadedInstance {
    application_bytes: Vec<u8>,
    application: ApplicationDigest,
    head: InstanceHead,
    records: Vec<(InstanceRecord, InstanceRecordDigest, usize)>,
}

pub struct InstanceStore {
    root: PathBuf,
    _lock: File,
}

impl InstanceStore {
    pub fn open(root: &Path) -> Result<Self> {
        validate_absolute_path(root, "instance store")?;
        validate_parent_chain(root, "instance store")?;
        match fs::symlink_metadata(root) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(io_error("instance store must be a non-symlink directory"));
                }
                ensure_private_directory(root)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                builder.mode(0o700);
                builder.create(root).map_err(LkError::from)?;
                sync_directory(
                    root.parent()
                        .ok_or_else(|| io_error("store has no parent"))?,
                )?;
            }
            Err(error) => return Err(error.into()),
        }
        let lock_path = root.join(LOCK_FILE);
        reject_symlink_if_present(&lock_path, "instance lock")?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&lock_path)?;
        lock.try_lock_exclusive().map_err(|_| {
            LkError::new(
                ErrorCode::AuthorityBusy,
                "another process owns the instance store authority",
            )
        })?;
        Ok(Self {
            root: root.to_path_buf(),
            _lock: lock,
        })
    }

    pub fn create(
        &self,
        request: &InstanceCreateRequest,
        application_bytes: &[u8],
    ) -> Result<InstanceCreateReceipt> {
        validate_version(request.version)?;
        validate_policy(request.policy)?;
        validate_grant(&request.grant, request.instance)?;
        let inspection = application::inspect(application_bytes)?;
        if !matches!(inspection.profile, InvocationProfile::Stateful(_)) {
            return Err(LkError::new(
                ErrorCode::RunArgumentMismatch,
                "instance creation requires a stateful application profile",
            ));
        }
        application::validate_stateful_state(application_bytes, &request.initial_state)?;
        check_value_bytes(
            &request.initial_state,
            request.policy.maximum_state_bytes,
            "initial state",
        )?;
        let grant = grant_digest(&request.grant)?;
        let state_digest = state_digest(inspection.digest, &request.initial_state)?;
        let record = InstanceRecord {
            instance: request.instance,
            application: inspection.digest,
            revision: 0,
            prior: None,
            state: request.initial_state.clone(),
            state_digest,
            response: ByteString::default(),
            event_key: None,
            input: None,
            command: None,
            grant_name: request.grant.name.clone(),
            grant,
            policy: request.policy,
        };
        let (record_bytes, record_digest) = encode_record(&record)?;
        let head = InstanceHead {
            instance: request.instance,
            revision: 0,
            record: record_digest,
            deleted: false,
        };
        let head_bytes = encode_envelope(HEAD_MAGIC, HEAD_DOMAIN, &head, 16 * 1024)?.0;
        let destination = self.instance_directory(request.instance);
        match fs::symlink_metadata(&destination) {
            Ok(_) => {
                return Err(LkError::new(
                    ErrorCode::WorkspaceExists,
                    "instance identity already exists or is tombstoned in this store",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let receipt = InstanceCreateReceipt {
            contract_version: INSTANCE_CONTRACT_VERSION,
            instance: request.instance,
            application: inspection.digest,
            revision: 0,
            state_digest,
            grant,
            published: request.mode == InstanceMode::Commit,
        };
        preflight_json(&receipt, "instance create receipt")?;
        if request.mode == InstanceMode::ValidateOnly {
            return Ok(receipt);
        }
        let temporary = self.root.join(format!(
            ".instance-{}-{}-{}",
            request.instance,
            std::process::id(),
            TEMPORARY_SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        create_private_directory(&temporary)?;
        let result = (|| -> Result<()> {
            create_private_directory(&temporary.join("records"))?;
            create_private_directory(&temporary.join("outcomes"))?;
            create_private_directory(&temporary.join("attempts"))?;
            write_new_file(&temporary.join(APPLICATION_FILE), application_bytes)?;
            write_new_file(
                &temporary
                    .join("records")
                    .join(record_file_name(0, record_digest)),
                &record_bytes,
            )?;
            write_new_file(&temporary.join(HEAD_FILE), &head_bytes)?;
            sync_directory(&temporary.join("records"))?;
            sync_directory(&temporary.join("outcomes"))?;
            sync_directory(&temporary.join("attempts"))?;
            sync_directory(&temporary)?;
            publish_instance_directory_with_fault(
                &self.root,
                &temporary,
                &destination,
                InstanceDirectoryFault::None,
            )?;
            Ok(())
        })();
        if let Err(error) = result {
            if fs::symlink_metadata(&destination).is_ok() {
                return Err(LkError::new(
                    ErrorCode::ArtifactPublicationOutcomeUnknown,
                    format!("instance creation may be visible: {error}"),
                ));
            }
            let _ = fs::remove_dir_all(&temporary);
            return Err(error);
        }
        Ok(receipt)
    }

    pub fn validate_event(
        &self,
        request: &InstanceEventRequest,
    ) -> Result<InstanceTransitionReceipt> {
        if request.mode != InstanceMode::ValidateOnly {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "validate-event requires validate_only mode",
            ));
        }
        self.prepare_event(request)
    }

    pub fn apply_event(&self, request: &InstanceEventRequest) -> Result<InstanceTransitionReceipt> {
        if request.mode != InstanceMode::Commit {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "apply-event requires commit mode",
            ));
        }
        self.prepare_event(request)
    }

    fn prepare_event(&self, request: &InstanceEventRequest) -> Result<InstanceTransitionReceipt> {
        validate_version(request.version)?;
        validate_event_key(request.mode, request.event_key.as_deref())?;
        let loaded = self.load(request.instance)?;
        reject_deleted(&loaded)?;
        let head = loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0
            .clone();
        let input = TransitionInput::External {
            base_revision: request.base_revision,
            event: request.event.clone(),
        };
        if let Some(receipt) = replay_receipt(&loaded, request.event_key.as_deref(), &input)? {
            return Ok(receipt);
        }
        if request.base_revision != loaded.head.revision {
            return Err(LkError::new(
                ErrorCode::RevisionConflict,
                format!(
                    "instance base revision {} is stale; current revision is {}",
                    request.base_revision, loaded.head.revision
                ),
            ));
        }
        if head.command.is_some() {
            return Err(LkError::new(
                ErrorCode::HostOutcomeUnknown,
                "instance has a pending host command; execute, reconcile, or resume it first",
            ));
        }
        check_value_bytes(
            &request.event,
            head.policy.maximum_event_bytes,
            "instance event",
        )?;
        let transition =
            application::transition_event(&loaded.application_bytes, &head.state, &request.event)?;
        self.finish_transition(
            &loaded,
            request.mode,
            request.event_key.clone(),
            input,
            transition,
        )
    }

    pub fn validate_resume(
        &self,
        request: &InstanceResumeRequest,
    ) -> Result<InstanceTransitionReceipt> {
        if request.mode != InstanceMode::ValidateOnly {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "validate-resume requires validate_only mode",
            ));
        }
        self.prepare_resume(request)
    }

    pub fn resume(&self, request: &InstanceResumeRequest) -> Result<InstanceTransitionReceipt> {
        if request.mode != InstanceMode::Commit {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "resume requires commit mode",
            ));
        }
        self.prepare_resume(request)
    }

    fn prepare_resume(&self, request: &InstanceResumeRequest) -> Result<InstanceTransitionReceipt> {
        validate_version(request.version)?;
        validate_event_key(request.mode, request.event_key.as_deref())?;
        let loaded = self.load(request.instance)?;
        reject_deleted(&loaded)?;
        if let Some(receipt) =
            replay_resume_receipt(&loaded, request.event_key.as_deref(), request.base_revision)?
        {
            return Ok(receipt);
        }
        let head = &loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0;
        let command = head.command.clone().ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "instance has no pending host command",
            )
        })?;
        let outcome = self.read_outcome(request.instance, &command)?;
        let input = TransitionInput::Host {
            base_revision: request.base_revision,
            command: command.id,
            outcome: outcome.outcome,
            evidence: outcome.evidence.clone(),
        };
        if request.base_revision != loaded.head.revision {
            return Err(LkError::new(
                ErrorCode::RevisionConflict,
                format!(
                    "instance base revision {} is stale; current revision is {}",
                    request.base_revision, loaded.head.revision
                ),
            ));
        }
        let transition = application::transition_resume(
            &loaded.application_bytes,
            &head.state,
            outcome.outcome,
            &outcome.evidence,
        )?;
        self.finish_transition(
            &loaded,
            request.mode,
            request.event_key.clone(),
            input,
            transition,
        )
    }

    pub fn execute_activation(
        &self,
        request: &InstanceHostRequest,
    ) -> Result<HostExecutionReceipt> {
        validate_version(request.version)?;
        let loaded = self.load(request.instance)?;
        reject_deleted(&loaded)?;
        let record = &loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0;
        let command =
            validate_host_request(record, request, StatefulCommandKind::ActivateApplication)?;
        require_executor(&request.grant, HostExecutorKind::Production)?;
        if let Some(outcome) = self.read_outcome_if_present(request.instance, &command)? {
            return Ok(host_receipt(outcome, true));
        }
        let source = request.source_application.as_deref().ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "activation execution requires one explicit source application path",
            )
        })?;
        let source = Path::new(source);
        validate_source_path(source, Path::new(&request.grant.source_directory))?;
        let source_bytes = application::read_file(source)?;
        let source_inspection = application::inspect(&source_bytes)?;
        if source_inspection.digest != command.application {
            return Err(LkError::new(
                ErrorCode::CapabilityDenied,
                "activation source does not match the exact requested application digest",
            ));
        }
        if self.attempt_exists(request.instance, command.id)? {
            let outcome = HostOutcomeRecord {
                instance: request.instance,
                command: command.id,
                outcome: HostOutcomeKind::OutcomeUnknown,
                evidence: ByteString::default(),
            };
            self.publish_outcome(&outcome)?;
            return Ok(host_receipt(outcome, false));
        }
        self.publish_attempt(HostAttemptRecord {
            instance: request.instance,
            command: command.id,
        })?;
        let outcome = match activate_slot(Path::new(&request.grant.slot), &source_bytes) {
            Ok(()) => HostOutcomeRecord {
                instance: request.instance,
                command: command.id,
                outcome: HostOutcomeKind::KnownSuccess,
                evidence: ByteString::from_slice(&command.application.as_bytes()).map_err(
                    |_| {
                        LkError::new(
                            ErrorCode::PolicyExceeded,
                            "activation evidence exceeds policy",
                        )
                    },
                )?,
            },
            Err(error) if error.code == ErrorCode::ArtifactPublicationOutcomeUnknown => {
                HostOutcomeRecord {
                    instance: request.instance,
                    command: command.id,
                    outcome: HostOutcomeKind::OutcomeUnknown,
                    evidence: ByteString::default(),
                }
            }
            Err(_) => HostOutcomeRecord {
                instance: request.instance,
                command: command.id,
                outcome: HostOutcomeKind::KnownFailureBeforeVisibility,
                evidence: ByteString::default(),
            },
        };
        self.publish_outcome(&outcome)?;
        Ok(host_receipt(outcome, false))
    }

    /// Validates an exact source artifact without making any external state visible.
    pub fn validate_application(
        &self,
        request: &InstanceHostRequest,
    ) -> Result<HostExecutionReceipt> {
        validate_version(request.version)?;
        let loaded = self.load(request.instance)?;
        reject_deleted(&loaded)?;
        let record = &loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0;
        let command =
            validate_host_request(record, request, StatefulCommandKind::ValidateApplication)?;
        require_executor(&request.grant, HostExecutorKind::Production)?;
        if let Some(outcome) = self.read_outcome_if_present(request.instance, &command)? {
            return Ok(host_receipt(outcome, true));
        }
        let source = request.source_application.as_deref().ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "application validation requires one explicit source application path",
            )
        })?;
        let source = Path::new(source);
        validate_source_path(source, Path::new(&request.grant.source_directory))?;
        let (outcome, evidence) =
            match application::read_file(source).and_then(|bytes| application::inspect(&bytes)) {
                Ok(inspection) if inspection.digest == command.application => (
                    HostOutcomeKind::KnownSuccess,
                    ByteString::from_slice(&command.application.as_bytes()).map_err(|_| {
                        LkError::new(
                            ErrorCode::PolicyExceeded,
                            "validation evidence exceeds policy",
                        )
                    })?,
                ),
                _ => (
                    HostOutcomeKind::KnownFailureBeforeVisibility,
                    ByteString::default(),
                ),
            };
        let result = HostOutcomeRecord {
            instance: request.instance,
            command: command.id,
            outcome,
            evidence,
        };
        self.publish_outcome(&result)?;
        Ok(host_receipt(result, false))
    }

    pub fn reconcile_activation(
        &self,
        request: &InstanceHostRequest,
    ) -> Result<HostExecutionReceipt> {
        validate_version(request.version)?;
        let loaded = self.load(request.instance)?;
        reject_deleted(&loaded)?;
        let record = &loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0;
        let command =
            validate_host_request(record, request, StatefulCommandKind::ReconcileActivation)?;
        require_executor(&request.grant, HostExecutorKind::Production)?;
        if let Some(outcome) = self.read_outcome_if_present(request.instance, &command)? {
            return Ok(host_receipt(outcome, true));
        }
        let slot = Path::new(&request.grant.slot);
        let (outcome, evidence) = match fs::symlink_metadata(slot) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (HostOutcomeKind::ReconciliationAbsent, ByteString::default())
            }
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                match application::read_file(slot).and_then(|bytes| application::inspect(&bytes)) {
                    Ok(inspection) if inspection.digest == command.application => (
                        HostOutcomeKind::ReconciliationPresent,
                        ByteString::from_slice(&command.application.as_bytes()).map_err(|_| {
                            LkError::new(
                                ErrorCode::PolicyExceeded,
                                "reconciliation evidence exceeds policy",
                            )
                        })?,
                    ),
                    _ => (
                        HostOutcomeKind::ReconciliationIndeterminate,
                        ByteString::default(),
                    ),
                }
            }
            _ => (
                HostOutcomeKind::ReconciliationIndeterminate,
                ByteString::default(),
            ),
        };
        let record = HostOutcomeRecord {
            instance: request.instance,
            command: command.id,
            outcome,
            evidence,
        };
        self.publish_outcome(&record)?;
        Ok(host_receipt(record, false))
    }

    /// Records one exact scripted outcome for an instance explicitly bound to the fake executor.
    pub fn record_fake_outcome(
        &self,
        request: &InstanceFakeHostRequest,
    ) -> Result<HostExecutionReceipt> {
        validate_version(request.version)?;
        let loaded = self.load(request.instance)?;
        reject_deleted(&loaded)?;
        let record = &loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0;
        let command =
            validate_host_binding(record, request.instance, request.command, &request.grant)?;
        require_executor(&request.grant, HostExecutorKind::DeterministicFake)?;
        if let Some(outcome) = self.read_outcome_if_present(request.instance, &command)? {
            let proposed = HostOutcomeRecord {
                instance: request.instance,
                command: command.id,
                outcome: request.outcome,
                evidence: request.evidence.clone(),
            };
            if outcome != proposed {
                return Err(LkError::new(
                    ErrorCode::IdempotencyConflict,
                    "fake host command already has a different exact outcome",
                ));
            }
            return Ok(host_receipt(outcome, true));
        }
        validate_host_outcome(&command, request.outcome, &request.evidence)?;
        let outcome = HostOutcomeRecord {
            instance: request.instance,
            command: command.id,
            outcome: request.outcome,
            evidence: request.evidence.clone(),
        };
        self.publish_outcome(&outcome)?;
        Ok(host_receipt(outcome, false))
    }

    pub fn inspect(&self, instance: InstanceId) -> Result<InstanceInspection> {
        let loaded = self.load(instance)?;
        let (record, digest, _) = loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?;
        let host_outcome = record
            .command
            .as_ref()
            .map(|command| self.read_outcome_if_present(instance, command))
            .transpose()?
            .flatten()
            .map(|outcome| outcome.outcome);
        let host_attempted = if host_outcome.is_none()
            && record
                .command
                .as_ref()
                .is_some_and(|command| command.kind == StatefulCommandKind::ActivateApplication)
        {
            self.attempt_exists(
                instance,
                record
                    .command
                    .as_ref()
                    .ok_or_else(|| corrupt("pending command disappeared during inspection"))?
                    .id,
            )?
        } else {
            false
        };
        let legal_actions = if loaded.head.deleted {
            vec!["inspect", "history"]
        } else if let Some(command) = &record.command {
            match (command.kind, host_outcome) {
                (_, Some(_)) => vec!["inspect", "history", "validate_resume", "resume"],
                (StatefulCommandKind::ValidateApplication, None) => {
                    vec!["inspect", "history", "validate_application"]
                }
                (StatefulCommandKind::ActivateApplication, None) => {
                    vec!["inspect", "history", "execute_activation"]
                }
                (StatefulCommandKind::ReconcileActivation, None) => {
                    vec!["inspect", "history", "reconcile_activation"]
                }
            }
        } else {
            vec![
                "inspect",
                "history",
                "validate_event",
                "apply_event",
                "delete",
            ]
        };
        Ok(InstanceInspection {
            contract_version: INSTANCE_CONTRACT_VERSION,
            format_version: INSTANCE_FORMAT_VERSION,
            instance,
            application: loaded.application,
            revision: record.revision,
            record_digest: *digest,
            state_digest: record.state_digest,
            state: record.state.clone(),
            response: record.response.clone(),
            pending_command: record.command.clone(),
            host_outcome,
            host_attempted,
            grant_name: record.grant_name.clone(),
            grant: record.grant,
            policy: record.policy,
            history_records: loaded.records.len() as u64,
            history_bytes: loaded.records.iter().map(|entry| entry.2 as u64).sum(),
            deleted: loaded.head.deleted,
            legal_actions,
        })
    }

    pub fn history(
        &self,
        instance: InstanceId,
        start_revision: u64,
        limit: usize,
    ) -> Result<InstanceHistoryPage> {
        if limit == 0 || limit > MAXIMUM_INSTANCE_HISTORY_PAGE {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "instance history page limit is outside policy",
            ));
        }
        let loaded = self.load(instance)?;
        let start = usize::try_from(start_revision).map_err(|_| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "history start revision overflows host indexes",
            )
        })?;
        if start > loaded.records.len() {
            return Err(LkError::new(
                ErrorCode::RevisionNotFound,
                "history start revision is absent",
            ));
        }
        let end = start.saturating_add(limit).min(loaded.records.len());
        let items = loaded.records[start..end]
            .iter()
            .map(|(record, digest, _)| InstanceHistoryItem {
                revision: record.revision,
                record_digest: *digest,
                state_digest: record.state_digest,
                event_key: record.event_key.clone(),
                status: if record.command.is_some() {
                    InstanceTransitionStatus::Suspended
                } else {
                    InstanceTransitionStatus::Completed
                },
                command: record.command.clone(),
            })
            .collect();
        Ok(InstanceHistoryPage {
            contract_version: INSTANCE_CONTRACT_VERSION,
            instance,
            start_revision,
            next_revision: end as u64,
            complete: end == loaded.records.len(),
            items,
        })
    }

    pub fn delete(&self, request: InstanceDeleteRequest) -> Result<InstanceInspection> {
        validate_version(request.version)?;
        let loaded = self.load(request.instance)?;
        reject_deleted(&loaded)?;
        if request.base_revision != loaded.head.revision {
            return Err(LkError::new(
                ErrorCode::RevisionConflict,
                "instance deletion base is stale",
            ));
        }
        let record = &loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0;
        if record.command.is_some() {
            return Err(LkError::new(
                ErrorCode::HostOutcomeUnknown,
                "instance with a pending command cannot be deleted",
            ));
        }
        let head = InstanceHead {
            deleted: true,
            ..loaded.head
        };
        self.publish_head(request.instance, &head)?;
        self.inspect(request.instance)
    }

    fn finish_transition(
        &self,
        loaded: &LoadedInstance,
        mode: InstanceMode,
        event_key: Option<String>,
        input: TransitionInput,
        transition: StatefulTransition,
    ) -> Result<InstanceTransitionReceipt> {
        let previous = &loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0;
        let next_revision = previous.revision.checked_add(1).ok_or_else(|| {
            LkError::new(ErrorCode::PolicyExceeded, "instance revision overflows")
        })?;
        check_transition_policy(previous, loaded, &transition, next_revision)?;
        let state_digest = state_digest(loaded.application, &transition.state)?;
        let command = transition.command.as_ref().map(|command| {
            pending_command(previous.instance, next_revision, previous.grant, command)
        });
        let record = InstanceRecord {
            instance: previous.instance,
            application: previous.application,
            revision: next_revision,
            prior: Some(loaded.head.record),
            state: transition.state,
            state_digest,
            response: transition.response,
            event_key,
            input: Some(input),
            command,
            grant_name: previous.grant_name.clone(),
            grant: previous.grant,
            policy: previous.policy,
        };
        let (bytes, digest) = encode_record(&record)?;
        let current_history_bytes: usize = loaded.records.iter().map(|entry| entry.2).sum();
        let maximum_history_bytes =
            usize::try_from(record.policy.maximum_history_bytes).unwrap_or(usize::MAX);
        if current_history_bytes.saturating_add(bytes.len()) > maximum_history_bytes {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "instance history byte policy would be exceeded",
            ));
        }
        let receipt = transition_receipt(&record, mode == InstanceMode::Commit, false);
        preflight_json(&receipt, "instance transition receipt")?;
        if mode == InstanceMode::ValidateOnly {
            return Ok(receipt);
        }
        self.publish_record(previous.instance, &record, digest, &bytes)?;
        Ok(receipt)
    }

    fn publish_record(
        &self,
        instance: InstanceId,
        record: &InstanceRecord,
        digest: InstanceRecordDigest,
        bytes: &[u8],
    ) -> Result<()> {
        let directory = self.instance_directory(instance);
        let records = directory.join("records");
        let final_path = records.join(record_file_name(record.revision, digest));
        publish_immutable(&records, &final_path, bytes, ".record-")?;
        let head = InstanceHead {
            instance,
            revision: record.revision,
            record: digest,
            deleted: false,
        };
        self.publish_head(instance, &head)
    }

    fn publish_head(&self, instance: InstanceId, head: &InstanceHead) -> Result<()> {
        let directory = self.instance_directory(instance);
        let bytes = encode_envelope(HEAD_MAGIC, HEAD_DOMAIN, head, 16 * 1024)?.0;
        publish_head_bytes_with_fault(&directory, &bytes, HeadPublicationFault::None)
    }

    fn load(&self, instance: InstanceId) -> Result<LoadedInstance> {
        let directory = self.instance_directory(instance);
        validate_instance_directory_layout(&directory)?;
        let head_bytes = read_bounded_file(&directory.join(HEAD_FILE), 16 * 1024, "instance HEAD")?;
        let head: InstanceHead =
            decode_envelope(HEAD_MAGIC, HEAD_DOMAIN, &head_bytes, 16 * 1024)?.0;
        if head.instance != instance {
            return Err(corrupt("instance HEAD has a foreign identity"));
        }
        let application_bytes = application::read_file(&directory.join(APPLICATION_FILE))?;
        let application_inspection = application::inspect(&application_bytes)?;
        let replay_limit = usize::try_from(head.revision)
            .ok()
            .and_then(|revision| revision.checked_add(1))
            .ok_or_else(|| {
                LkError::new(ErrorCode::PolicyExceeded, "instance replay count overflows")
            })?;
        if replay_limit > MAXIMUM_INSTANCE_REPLAY_WORK {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "instance replay work exceeds global policy",
            ));
        }
        let mut records = Vec::with_capacity(replay_limit);
        let mut digest = head.record;
        for revision in (0..=head.revision).rev() {
            let path = directory
                .join("records")
                .join(record_file_name(revision, digest));
            let bytes = read_bounded_file(&path, MAXIMUM_INSTANCE_RECORD_BYTES, "instance record")?;
            let (record, actual_digest): (InstanceRecord, InstanceRecordDigest) =
                decode_record(&bytes)?;
            if actual_digest != digest || record.revision != revision {
                return Err(corrupt("instance record chain identity is inconsistent"));
            }
            digest = record
                .prior
                .unwrap_or_else(|| InstanceRecordDigest::from_bytes([0; 32]));
            records.push((record, actual_digest, bytes.len()));
        }
        records.reverse();
        validate_history(
            instance,
            application_inspection.digest,
            &application_bytes,
            &records,
        )?;
        Ok(LoadedInstance {
            application_bytes,
            application: application_inspection.digest,
            head,
            records,
        })
    }

    fn read_outcome(
        &self,
        instance: InstanceId,
        command: &PendingCommand,
    ) -> Result<HostOutcomeRecord> {
        let path = self
            .instance_directory(instance)
            .join("outcomes")
            .join(format!("{}.lkio", command.id));
        let bytes = read_bounded_file(&path, MAXIMUM_HOST_EVIDENCE_BYTES * 2, "host outcome")?;
        let (outcome, _): (HostOutcomeRecord, InstanceRecordDigest) = decode_envelope(
            OUTCOME_MAGIC,
            OUTCOME_DOMAIN,
            &bytes,
            MAXIMUM_HOST_EVIDENCE_BYTES * 2,
        )?;
        if outcome.instance != instance || outcome.command != command.id {
            return Err(corrupt("host outcome has a foreign command domain"));
        }
        validate_host_outcome(command, outcome.outcome, &outcome.evidence)
            .map_err(|_| corrupt("host outcome is incompatible with the pending command"))?;
        Ok(outcome)
    }

    fn read_outcome_if_present(
        &self,
        instance: InstanceId,
        command: &PendingCommand,
    ) -> Result<Option<HostOutcomeRecord>> {
        let path = self
            .instance_directory(instance)
            .join("outcomes")
            .join(format!("{}.lkio", command.id));
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
            Ok(_) => self.read_outcome(instance, command).map(Some),
        }
    }

    fn publish_outcome(&self, outcome: &HostOutcomeRecord) -> Result<()> {
        let directory = self.instance_directory(outcome.instance).join("outcomes");
        let path = directory.join(format!("{}.lkio", outcome.command));
        let bytes = encode_envelope(
            OUTCOME_MAGIC,
            OUTCOME_DOMAIN,
            outcome,
            MAXIMUM_HOST_EVIDENCE_BYTES * 2,
        )?
        .0;
        publish_immutable(&directory, &path, &bytes, ".outcome-")
    }

    fn attempt_exists(&self, instance: InstanceId, command: CommandId) -> Result<bool> {
        let path = self
            .instance_directory(instance)
            .join("attempts")
            .join(format!("{command}.lkia"));
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                let bytes = read_bounded_file(&path, 2 * 1024, "host attempt marker")?;
                let (attempt, _): (HostAttemptRecord, InstanceRecordDigest) =
                    decode_envelope(ATTEMPT_MAGIC, ATTEMPT_DOMAIN, &bytes, 1024)?;
                if attempt.instance != instance || attempt.command != command {
                    return Err(corrupt("host attempt marker has a foreign command domain"));
                }
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn publish_attempt(&self, attempt: HostAttemptRecord) -> Result<()> {
        let directory = self.instance_directory(attempt.instance).join("attempts");
        let path = directory.join(format!("{}.lkia", attempt.command));
        let bytes = encode_envelope(ATTEMPT_MAGIC, ATTEMPT_DOMAIN, &attempt, 1024)?.0;
        publish_immutable(&directory, &path, &bytes, ".attempt-")
    }

    fn instance_directory(&self, instance: InstanceId) -> PathBuf {
        self.root.join(instance.to_string())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)] // Non-None variants are deterministic crash-test injection points.
enum HeadPublicationFault {
    None,
    BeforeWrite,
    AfterWrite,
    AfterFileSync,
    AfterVisibility,
    AfterDirectorySync,
}

fn publish_head_bytes_with_fault(
    directory: &Path,
    bytes: &[u8],
    fault: HeadPublicationFault,
) -> Result<()> {
    let temporary = directory.join(format!(
        ".head-{}-{}",
        std::process::id(),
        TEMPORARY_SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let file_fault = match fault {
        HeadPublicationFault::BeforeWrite => DurableFileFault::BeforeWrite,
        HeadPublicationFault::AfterWrite => DurableFileFault::AfterWrite,
        HeadPublicationFault::AfterFileSync => DurableFileFault::AfterFileSync,
        _ => DurableFileFault::None,
    };
    if let Err(error) = write_new_file_with_fault(&temporary, bytes, file_fault) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, directory.join(HEAD_FILE)) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    if fault == HeadPublicationFault::AfterVisibility {
        return Err(unknown(
            "instance HEAD became visible before directory synchronization",
        ));
    }
    sync_directory(directory).map_err(|error| {
        unknown(&format!(
            "instance HEAD may be visible but directory sync failed: {error}"
        ))
    })?;
    if fault == HeadPublicationFault::AfterDirectorySync {
        return Err(unknown(
            "instance HEAD became durable but its result was not acknowledged",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstanceDirectoryFault {
    None,
    BeforeVisibility,
    AfterVisibility,
    AfterDirectorySync,
}

fn publish_instance_directory_with_fault(
    root: &Path,
    temporary: &Path,
    destination: &Path,
    fault: InstanceDirectoryFault,
) -> Result<()> {
    if fault == InstanceDirectoryFault::BeforeVisibility {
        return Err(injected("instance directory before visibility"));
    }
    fs::rename(temporary, destination)?;
    if fault == InstanceDirectoryFault::AfterVisibility {
        return Err(unknown(
            "instance directory became visible before store synchronization",
        ));
    }
    sync_directory(root).map_err(|error| {
        unknown(&format!(
            "instance directory may be visible but store sync failed: {error}"
        ))
    })?;
    if fault == InstanceDirectoryFault::AfterDirectorySync {
        return Err(unknown(
            "instance directory became durable but its result was not acknowledged",
        ));
    }
    Ok(())
}

fn validate_history(
    instance: InstanceId,
    application_digest: ApplicationDigest,
    application_bytes: &[u8],
    records: &[(InstanceRecord, InstanceRecordDigest, usize)],
) -> Result<()> {
    if records.is_empty() || records.len() > MAXIMUM_INSTANCE_REPLAY_WORK {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "instance history replay is outside policy",
        ));
    }
    let mut prior: Option<&InstanceRecord> = None;
    let mut prior_digest = None;
    let mut event_keys = std::collections::BTreeSet::new();
    for (index, (record, digest, _)) in records.iter().enumerate() {
        if record.instance != instance || record.application != application_digest {
            return Err(corrupt("instance record belongs to a foreign authority"));
        }
        if record.revision != index as u64 || record.prior != prior_digest {
            return Err(corrupt(
                "instance revisions are not one contiguous exact chain",
            ));
        }
        validate_policy(record.policy)?;
        validate_event_key(
            if record.event_key.is_some() {
                InstanceMode::Commit
            } else {
                InstanceMode::ValidateOnly
            },
            record.event_key.as_deref(),
        )?;
        if let Some(key) = &record.event_key
            && !event_keys.insert(key.clone())
        {
            return Err(corrupt("instance event keys are not unique"));
        }
        application::validate_stateful_state(application_bytes, &record.state)?;
        check_value_bytes(
            &record.state,
            record.policy.maximum_state_bytes,
            "retained state",
        )?;
        if state_digest(application_digest, &record.state)? != record.state_digest {
            return Err(corrupt("instance state digest is invalid"));
        }
        match (prior, &record.input) {
            (None, None) if record.revision == 0 && record.command.is_none() => {}
            (Some(previous), Some(input)) => {
                let transition = match input {
                    TransitionInput::External {
                        base_revision,
                        event,
                    } => {
                        if *base_revision != previous.revision {
                            return Err(corrupt("retained external event has the wrong base"));
                        }
                        application::transition_event(application_bytes, &previous.state, event)?
                    }
                    TransitionInput::Host {
                        base_revision,
                        command,
                        outcome,
                        evidence,
                    } => {
                        let pending = previous.command.as_ref().ok_or_else(|| {
                            corrupt("retained host outcome has no pending command")
                        })?;
                        if *base_revision != previous.revision || pending.id != *command {
                            return Err(corrupt(
                                "retained host outcome has the wrong pending command",
                            ));
                        }
                        validate_host_outcome(pending, *outcome, evidence).map_err(|_| {
                            corrupt("retained host outcome is incompatible with its command")
                        })?;
                        application::transition_resume(
                            application_bytes,
                            &previous.state,
                            *outcome,
                            evidence,
                        )?
                    }
                };
                let expected_command = transition.command.as_ref().map(|command| {
                    pending_command(instance, record.revision, record.grant, command)
                });
                if transition.state != record.state
                    || transition.response != record.response
                    || expected_command != record.command
                {
                    return Err(corrupt(
                        "instance replay does not reproduce the retained transition",
                    ));
                }
                if record.grant != previous.grant
                    || record.grant_name != previous.grant_name
                    || record.policy != previous.policy
                {
                    return Err(corrupt(
                        "instance immutable policy or grant changed across history",
                    ));
                }
            }
            _ => return Err(corrupt("instance transition history shape is invalid")),
        }
        prior = Some(record);
        prior_digest = Some(*digest);
    }
    Ok(())
}

fn check_transition_policy(
    previous: &InstanceRecord,
    loaded: &LoadedInstance,
    transition: &StatefulTransition,
    next_revision: u64,
) -> Result<()> {
    let maximum_transitions = previous.policy.maximum_transitions;
    if next_revision > maximum_transitions || next_revision as usize > MAXIMUM_INSTANCE_TRANSITIONS
    {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "instance transition count exceeds policy",
        ));
    }
    if loaded.records.len().saturating_add(1)
        > usize::try_from(previous.policy.maximum_replay_work).unwrap_or(usize::MAX)
    {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "instance replay work would exceed policy",
        ));
    }
    check_value_bytes(
        &transition.state,
        previous.policy.maximum_state_bytes,
        "next state",
    )?;
    if transition.response.len() > MAXIMUM_HOST_EVIDENCE_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "transition response exceeds policy",
        ));
    }
    Ok(())
}

fn pending_command(
    instance: InstanceId,
    revision: u64,
    grant: ActivationGrantDigest,
    command: &StatefulCommand,
) -> PendingCommand {
    let mut hasher = blake3::Hasher::new_derive_key(COMMAND_DOMAIN);
    hasher.update(&instance.as_bytes());
    hasher.update(&revision.to_le_bytes());
    hasher.update(&grant.as_bytes());
    hasher.update(&command.application.as_bytes());
    hasher.update(&[match command.kind {
        StatefulCommandKind::ValidateApplication => 1,
        StatefulCommandKind::ActivateApplication => 2,
        StatefulCommandKind::ReconcileActivation => 3,
    }]);
    PendingCommand {
        id: CommandId::from_bytes(*hasher.finalize().as_bytes()),
        kind: command.kind,
        application: command.application,
        grant,
    }
}

fn replay_receipt(
    loaded: &LoadedInstance,
    key: Option<&str>,
    input: &TransitionInput,
) -> Result<Option<InstanceTransitionReceipt>> {
    let Some(key) = key else {
        return Ok(None);
    };
    let Some((record, _, _)) = loaded
        .records
        .iter()
        .find(|(record, _, _)| record.event_key.as_deref() == Some(key))
    else {
        return Ok(None);
    };
    if record.input.as_ref() != Some(input) {
        return Err(LkError::new(
            ErrorCode::IdempotencyConflict,
            "instance event key was already bound to different canonical input",
        ));
    }
    Ok(Some(transition_receipt(record, true, true)))
}

fn replay_resume_receipt(
    loaded: &LoadedInstance,
    key: Option<&str>,
    base_revision: u64,
) -> Result<Option<InstanceTransitionReceipt>> {
    let Some(key) = key else {
        return Ok(None);
    };
    let Some((record, _, _)) = loaded
        .records
        .iter()
        .find(|(record, _, _)| record.event_key.as_deref() == Some(key))
    else {
        return Ok(None);
    };
    if !matches!(
        record.input,
        Some(TransitionInput::Host {
            base_revision: retained,
            ..
        }) if retained == base_revision
    ) {
        return Err(LkError::new(
            ErrorCode::IdempotencyConflict,
            "instance event key was already bound to a different resume request",
        ));
    }
    Ok(Some(transition_receipt(record, true, true)))
}

fn transition_receipt(
    record: &InstanceRecord,
    published: bool,
    replayed: bool,
) -> InstanceTransitionReceipt {
    let base_revision = record.revision.saturating_sub(1);
    InstanceTransitionReceipt {
        contract_version: INSTANCE_CONTRACT_VERSION,
        instance: record.instance,
        application: record.application,
        base_revision,
        next_revision: record.revision,
        state_digest: record.state_digest,
        response: record.response.clone(),
        status: if record.command.is_some() {
            InstanceTransitionStatus::Suspended
        } else {
            InstanceTransitionStatus::Completed
        },
        command: record.command.clone(),
        published,
        replayed,
    }
}

fn validate_host_request(
    record: &InstanceRecord,
    request: &InstanceHostRequest,
    expected: StatefulCommandKind,
) -> Result<PendingCommand> {
    let command = validate_host_binding(record, request.instance, request.command, &request.grant)?;
    if command.kind != expected {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "host request has the wrong operation for the exact pending command",
        ));
    }
    Ok(command)
}

fn validate_host_binding(
    record: &InstanceRecord,
    instance: InstanceId,
    command: CommandId,
    grant: &ActivationGrant,
) -> Result<PendingCommand> {
    validate_grant(grant, instance)?;
    let digest = grant_digest(grant)?;
    if digest != record.grant || grant.name != record.grant_name {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "supplied activation grant does not match the instance grant requirement",
        ));
    }
    let pending = record.command.clone().ok_or_else(|| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            "instance has no pending host command",
        )
    })?;
    if pending.id != command {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "host request does not match the exact pending command",
        ));
    }
    Ok(pending)
}

fn require_executor(grant: &ActivationGrant, expected: HostExecutorKind) -> Result<()> {
    if grant.executor == expected {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "activation grant is bound to a different host executor",
        ))
    }
}

fn validate_host_outcome(
    command: &PendingCommand,
    outcome: HostOutcomeKind,
    evidence: &ByteString,
) -> Result<()> {
    let compatible = match command.kind {
        StatefulCommandKind::ValidateApplication => matches!(
            outcome,
            HostOutcomeKind::KnownSuccess
                | HostOutcomeKind::KnownFailureBeforeVisibility
                | HostOutcomeKind::CancelledBeforeAction
                | HostOutcomeKind::TimeoutBeforeAction
        ),
        StatefulCommandKind::ActivateApplication => matches!(
            outcome,
            HostOutcomeKind::KnownSuccess
                | HostOutcomeKind::KnownFailureBeforeVisibility
                | HostOutcomeKind::OutcomeUnknown
                | HostOutcomeKind::CancelledBeforeAction
                | HostOutcomeKind::TimeoutBeforeAction
                | HostOutcomeKind::TimeoutAfterPossibleVisibility
                | HostOutcomeKind::CleanupFailure
        ),
        StatefulCommandKind::ReconcileActivation => matches!(
            outcome,
            HostOutcomeKind::ReconciliationPresent
                | HostOutcomeKind::ReconciliationAbsent
                | HostOutcomeKind::ReconciliationIndeterminate
        ),
    };
    if !compatible {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "fake host outcome is incompatible with the pending command kind",
        ));
    }
    let needs_digest = matches!(
        outcome,
        HostOutcomeKind::KnownSuccess | HostOutcomeKind::ReconciliationPresent
    );
    if needs_digest {
        if evidence.as_slice() != command.application.as_bytes() {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "successful fake host evidence must be the exact target application digest",
            ));
        }
    } else if !evidence.is_empty() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "non-success fake host outcomes require empty evidence",
        ));
    }
    Ok(())
}

fn host_receipt(record: HostOutcomeRecord, replayed: bool) -> HostExecutionReceipt {
    HostExecutionReceipt {
        contract_version: INSTANCE_CONTRACT_VERSION,
        instance: record.instance,
        command: record.command,
        outcome: record.outcome,
        evidence: record.evidence,
        replayed,
    }
}

fn grant_digest(grant: &ActivationGrant) -> Result<ActivationGrantDigest> {
    let bytes = canonical_json(grant, "activation grant")?;
    let mut hasher = blake3::Hasher::new_derive_key(GRANT_DOMAIN);
    hasher.update(&bytes);
    Ok(ActivationGrantDigest::from_bytes(
        *hasher.finalize().as_bytes(),
    ))
}

fn state_digest(application: ApplicationDigest, state: &ApplicationValue) -> Result<StateDigest> {
    let bytes = canonical_json(state, "instance state")?;
    let mut hasher = blake3::Hasher::new_derive_key(STATE_DOMAIN);
    hasher.update(&application.as_bytes());
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(&bytes);
    Ok(StateDigest::from_bytes(*hasher.finalize().as_bytes()))
}

fn encode_record(record: &InstanceRecord) -> Result<(Vec<u8>, InstanceRecordDigest)> {
    encode_envelope(
        RECORD_MAGIC,
        RECORD_DOMAIN,
        record,
        MAXIMUM_INSTANCE_RECORD_BYTES,
    )
}

fn decode_record(bytes: &[u8]) -> Result<(InstanceRecord, InstanceRecordDigest)> {
    decode_envelope(
        RECORD_MAGIC,
        RECORD_DOMAIN,
        bytes,
        MAXIMUM_INSTANCE_RECORD_BYTES,
    )
}

fn encode_envelope<T: Serialize>(
    magic: [u8; 8],
    domain: &str,
    value: &T,
    maximum: usize,
) -> Result<(Vec<u8>, InstanceRecordDigest)> {
    let payload = canonical_json(value, "instance artifact payload")?;
    if payload.len() > maximum {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "instance artifact payload exceeds policy",
        ));
    }
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&payload);
    let digest = InstanceRecordDigest::from_bytes(*hasher.finalize().as_bytes());
    let mut writer = Writer::with_capacity(8 + 2 + 8 + payload.len() + 32);
    writer.fixed(&magic);
    writer.u16(INSTANCE_FORMAT_VERSION);
    writer.u64(payload.len() as u64);
    writer.fixed(&payload);
    writer.fixed(&digest.as_bytes());
    let bytes = writer.finish();
    if bytes.len() > maximum.saturating_add(50) {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "instance artifact envelope exceeds policy",
        ));
    }
    Ok((bytes, digest))
}

fn decode_envelope<T: DeserializeOwned + Serialize>(
    magic: [u8; 8],
    domain: &str,
    bytes: &[u8],
    maximum: usize,
) -> Result<(T, InstanceRecordDigest)> {
    if bytes.len() > maximum.saturating_add(50) {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "instance artifact exceeds decoder policy",
        ));
    }
    let mut reader = Reader::new(bytes);
    if reader.fixed(8).map_err(instance_codec)? != magic {
        return Err(corrupt("instance artifact magic is invalid"));
    }
    if reader.u16().map_err(instance_codec)? != INSTANCE_FORMAT_VERSION {
        return Err(corrupt("instance artifact version is unsupported"));
    }
    let length = usize::try_from(reader.u64().map_err(instance_codec)?)
        .map_err(|_| corrupt("instance artifact length overflows host indexes"))?;
    if length > maximum {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "instance artifact payload exceeds decoder policy",
        ));
    }
    let payload = reader.fixed(length).map_err(instance_codec)?;
    let encoded_digest = reader.fixed(32).map_err(instance_codec)?;
    reader.finish().map_err(instance_codec)?;
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(payload);
    let digest = InstanceRecordDigest::from_bytes(*hasher.finalize().as_bytes());
    if encoded_digest != digest.as_bytes() {
        return Err(corrupt("instance artifact digest is invalid"));
    }
    let value = strict_json::<T>(payload, "instance artifact payload")?;
    if canonical_json(&value, "instance artifact payload")? != payload {
        return Err(corrupt("instance artifact payload is not canonical JSON"));
    }
    Ok((value, digest))
}

pub fn strict_json<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = T::deserialize(&mut deserializer).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} JSON is malformed: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} JSON has trailing input: {error}"),
        )
    })?;
    Ok(value)
}

fn canonical_json<T: Serialize>(value: &T, label: &str) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode {label}: {error}"),
        )
    })
}

fn preflight_json<T: Serialize>(value: &T, label: &str) -> Result<()> {
    let bytes = canonical_json(value, label)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds output policy"),
        ));
    }
    Ok(())
}

fn validate_version(version: u16) -> Result<()> {
    if version == INSTANCE_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::ProtocolVersion,
            format!(
                "instance contract version {version} is unsupported; expected {INSTANCE_CONTRACT_VERSION}"
            ),
        ))
    }
}

fn validate_policy(policy: InstancePolicy) -> Result<()> {
    let valid = policy.maximum_state_bytes > 0
        && policy.maximum_state_bytes <= MAXIMUM_INSTANCE_STATE_BYTES as u64
        && policy.maximum_event_bytes > 0
        && policy.maximum_event_bytes <= MAXIMUM_INSTANCE_EVENT_BYTES as u64
        && policy.maximum_history_bytes > 0
        && policy.maximum_history_bytes <= MAXIMUM_INSTANCE_HISTORY_BYTES as u64
        && policy.maximum_transitions > 0
        && policy.maximum_transitions <= MAXIMUM_INSTANCE_TRANSITIONS as u64
        && policy.maximum_replay_work > 0
        && policy.maximum_replay_work <= MAXIMUM_INSTANCE_REPLAY_WORK as u64;
    if valid {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "instance policy is outside global bounds",
        ))
    }
}

fn validate_event_key(mode: InstanceMode, key: Option<&str>) -> Result<()> {
    match (mode, key) {
        (InstanceMode::ValidateOnly, None) => Ok(()),
        (InstanceMode::ValidateOnly, Some(_)) => Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "validate-only instance requests cannot consume an event key",
        )),
        (InstanceMode::Commit, Some(key))
            if !key.is_empty()
                && key.len() <= MAXIMUM_EVENT_KEY_BYTES
                && key.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
                }) =>
        {
            Ok(())
        }
        (InstanceMode::Commit, _) => Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "committed instance events require a bounded canonical event key",
        )),
    }
}

fn validate_grant(grant: &ActivationGrant, instance: InstanceId) -> Result<()> {
    validate_version(grant.version)?;
    if grant.instance != instance {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "activation grant is bound to another instance",
        ));
    }
    if grant.name.is_empty()
        || grant.name.len() > MAXIMUM_GRANT_NAME_BYTES
        || !grant.name.as_bytes()[0].is_ascii_lowercase()
        || !grant
            .name
            .bytes()
            .skip(1)
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "activation grant name is not canonical",
        ));
    }
    let source = Path::new(&grant.source_directory);
    let slot = Path::new(&grant.slot);
    validate_absolute_path(source, "activation source directory")?;
    validate_parent_chain(source, "activation source directory")?;
    let metadata = fs::symlink_metadata(source).map_err(LkError::from)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "activation source grant is not a regular directory",
        ));
    }
    validate_absolute_path(slot, "activation slot")?;
    validate_parent_chain(slot, "activation slot")?;
    if let Ok(metadata) = fs::symlink_metadata(slot)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "activation slot is not a regular file",
        ));
    }
    Ok(())
}

fn check_value_bytes(value: &ApplicationValue, maximum: u64, label: &str) -> Result<()> {
    let bytes = canonical_json(value, label)?;
    if bytes.len() as u64 > maximum {
        Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds byte policy"),
        ))
    } else {
        Ok(())
    }
}

fn reject_deleted(loaded: &LoadedInstance) -> Result<()> {
    if loaded.head.deleted {
        Err(LkError::new(
            ErrorCode::DeleteBlocked,
            "instance identity is tombstoned and cannot be reused",
        ))
    } else {
        Ok(())
    }
}

fn record_file_name(revision: u64, digest: InstanceRecordDigest) -> String {
    format!("{revision:020}-{digest}.lkis")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActivationFault {
    None,
    BeforeWrite,
    AfterWrite,
    AfterFileSync,
    AfterVisibility,
    AfterDirectorySync,
}

fn activate_slot(slot: &Path, bytes: &[u8]) -> Result<()> {
    activate_slot_with_fault(slot, bytes, ActivationFault::None)
}

fn activate_slot_with_fault(slot: &Path, bytes: &[u8], fault: ActivationFault) -> Result<()> {
    validate_absolute_path(slot, "activation slot")?;
    validate_parent_chain(slot, "activation slot")?;
    let parent = slot
        .parent()
        .ok_or_else(|| io_error("activation slot has no parent"))?;
    let temporary = parent.join(format!(
        ".activation-{}-{}",
        std::process::id(),
        TEMPORARY_SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    if fault == ActivationFault::BeforeWrite {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(injected("activation before write"));
    }
    if let Err(error) = file.write_all(bytes) {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    if fault == ActivationFault::AfterWrite {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(injected("activation after write"));
    }
    if let Err(error) = file.sync_all() {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    drop(file);
    if fault == ActivationFault::AfterFileSync {
        let _ = fs::remove_file(&temporary);
        return Err(injected("activation after file sync"));
    }
    if let Err(error) = fs::rename(&temporary, slot) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    if fault == ActivationFault::AfterVisibility {
        return Err(unknown("activation became visible before directory sync"));
    }
    sync_directory(parent).map_err(|error| {
        unknown(&format!(
            "activation became visible but directory sync failed: {error}"
        ))
    })?;
    if fault == ActivationFault::AfterDirectorySync {
        return Err(unknown(
            "activation became durable but its result was not acknowledged",
        ));
    }
    Ok(())
}

fn validate_source_path(path: &Path, directory: &Path) -> Result<()> {
    validate_absolute_path(path, "activation source application")?;
    validate_parent_chain(path, "activation source application")?;
    if path == directory || !path.starts_with(directory) {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "activation source escapes the granted directory",
        ));
    }
    Ok(())
}

fn publish_immutable(
    directory: &Path,
    final_path: &Path,
    bytes: &[u8],
    prefix: &str,
) -> Result<()> {
    publish_immutable_with_fault(
        directory,
        final_path,
        bytes,
        prefix,
        ImmutablePublicationFault::None,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)] // Non-None variants are deterministic crash-test injection points.
enum ImmutablePublicationFault {
    None,
    BeforeWrite,
    AfterWrite,
    AfterFileSync,
    AfterLink,
    AfterCleanup,
    AfterDirectorySync,
}

fn publish_immutable_with_fault(
    directory: &Path,
    final_path: &Path,
    bytes: &[u8],
    prefix: &str,
    fault: ImmutablePublicationFault,
) -> Result<()> {
    if let Ok(existing) = read_bounded_file(final_path, bytes.len(), "existing immutable object") {
        if existing == bytes {
            return Ok(());
        }
        return Err(corrupt("conflicting bytes claim one immutable object name"));
    }
    let temporary = directory.join(format!(
        "{prefix}{}-{}",
        std::process::id(),
        TEMPORARY_SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let file_fault = match fault {
        ImmutablePublicationFault::BeforeWrite => DurableFileFault::BeforeWrite,
        ImmutablePublicationFault::AfterWrite => DurableFileFault::AfterWrite,
        ImmutablePublicationFault::AfterFileSync => DurableFileFault::AfterFileSync,
        _ => DurableFileFault::None,
    };
    if let Err(error) = write_new_file_with_fault(&temporary, bytes, file_fault) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    match fs::hard_link(&temporary, final_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = read_bounded_file(final_path, bytes.len(), "existing immutable object")?;
            if existing != bytes {
                let _ = fs::remove_file(&temporary);
                return Err(corrupt(
                    "conflicting immutable object appeared during publication",
                ));
            }
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
    }
    if fault == ImmutablePublicationFault::AfterLink {
        return Err(unknown(
            "immutable object became visible before temporary cleanup",
        ));
    }
    fs::remove_file(&temporary).map_err(|error| {
        unknown(&format!(
            "immutable object linked but temporary cleanup failed: {error}"
        ))
    })?;
    if fault == ImmutablePublicationFault::AfterCleanup {
        return Err(unknown(
            "immutable object became visible before directory synchronization",
        ));
    }
    sync_directory(directory).map_err(|error| {
        unknown(&format!(
            "immutable object linked but directory sync failed: {error}"
        ))
    })?;
    if fault == ImmutablePublicationFault::AfterDirectorySync {
        return Err(unknown(
            "immutable object became durable but its result was not acknowledged",
        ));
    }
    Ok(())
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<()> {
    write_new_file_with_fault(path, bytes, DurableFileFault::None)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DurableFileFault {
    None,
    BeforeWrite,
    AfterWrite,
    AfterFileSync,
}

fn write_new_file_with_fault(path: &Path, bytes: &[u8], fault: DurableFileFault) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    if fault == DurableFileFault::BeforeWrite {
        return Err(injected("durable file before write"));
    }
    file.write_all(bytes)?;
    if fault == DurableFileFault::AfterWrite {
        return Err(injected("durable file after write"));
    }
    file.sync_all()?;
    if fault == DurableFileFault::AfterFileSync {
        return Err(injected("durable file after file sync"));
    }
    Ok(())
}

fn read_bounded_file(path: &Path, maximum: usize, label: &str) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| LkError::new(ErrorCode::Io, format!("cannot inspect {label}: {error}")))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(corrupt(&format!(
            "{label} is not a regular non-symlink file"
        )));
    }
    if metadata.len() > maximum as u64 {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds byte policy"),
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds byte policy"),
        ));
    }
    Ok(bytes)
}

fn validate_absolute_path(path: &Path, label: &str) -> Result<()> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.is_empty() || bytes.len() > MAXIMUM_INSTANCE_PATH_BYTES || !path.is_absolute() {
        return Err(LkError::new(
            ErrorCode::Io,
            format!("{label} path is not a bounded absolute path"),
        ));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::RootDir | Component::Normal(_)))
    {
        return Err(LkError::new(
            ErrorCode::Io,
            format!("{label} path contains a noncanonical component"),
        ));
    }
    if bytes[1..]
        .split(|byte| *byte == b'/')
        .any(|component| component.is_empty())
    {
        return Err(LkError::new(
            ErrorCode::Io,
            format!("{label} path contains repeated separators"),
        ));
    }
    Ok(())
}

fn validate_parent_chain(path: &Path, label: &str) -> Result<()> {
    let mut current = PathBuf::from("/");
    let components: Vec<_> = path.components().collect();
    for component in components
        .iter()
        .skip(1)
        .take(components.len().saturating_sub(2))
    {
        if let Component::Normal(part) = component {
            current.push(part);
            let metadata = fs::symlink_metadata(&current).map_err(|error| {
                LkError::new(
                    ErrorCode::Io,
                    format!("cannot inspect {label} parent: {error}"),
                )
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LkError::new(
                    ErrorCode::Io,
                    format!("{label} parent is not a regular directory"),
                ));
            }
        }
    }
    Ok(())
}

fn reject_symlink_if_present(path: &Path, label: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(LkError::new(
            ErrorCode::Io,
            format!("{label} cannot be a symlink"),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn create_private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    builder.create(path)?;
    Ok(())
}

fn ensure_private_directory(path: &Path) -> Result<()> {
    let mode = fs::metadata(path)?.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(LkError::new(
            ErrorCode::Io,
            "instance store permissions must exclude group and other access",
        ));
    }
    Ok(())
}

fn validate_instance_directory_layout(directory: &Path) -> Result<()> {
    for (path, label) in [
        (directory.to_path_buf(), "instance directory"),
        (directory.join("records"), "instance records directory"),
        (directory.join("outcomes"), "instance outcomes directory"),
        (directory.join("attempts"), "instance attempts directory"),
    ] {
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                format!("{label} is absent: {error}"),
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(corrupt(&format!(
                "{label} is not a regular non-symlink directory"
            )));
        }
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(corrupt(&format!(
                "{label} permissions expose instance authority"
            )));
        }
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn instance_codec(error: CodecError) -> LkError {
    LkError::new(
        ErrorCode::ArtifactCorrupt,
        format!("instance artifact is malformed: {error}"),
    )
}

fn corrupt(message: &str) -> LkError {
    LkError::new(ErrorCode::ArtifactCorrupt, message)
}

fn io_error(message: &str) -> LkError {
    LkError::new(ErrorCode::Io, message)
}

fn injected(message: &str) -> LkError {
    LkError::new(ErrorCode::Io, format!("injected fault: {message}"))
}

fn unknown(message: &str) -> LkError {
    LkError::new(ErrorCode::ArtifactPublicationOutcomeUnknown, message)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

fn decode_hex<const N: usize>(value: &str) -> std::result::Result<[u8; N], &'static str> {
    if value.len() != N * 2
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
    {
        return Err("hex value is not canonical lowercase fixed-width input");
    }
    let mut output = [0_u8; N];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0]).ok_or("hex digit is invalid")?;
        let low = hex_nibble(pair[1]).ok_or("hex digit is invalid")?;
        output[index] = (high << 4) | low;
    }
    Ok(output)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
