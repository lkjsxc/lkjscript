//! Durable typed application instances and exact host-interface adapters.

use crate::application::{
    self, ApplicationDigest, ApplicationLoadObservation, ApplicationValue, HostInterface,
    HostInterfaceId, HostOperation, HostOutcomeClass, InvocationProfile, StatefulCommand,
    StatefulTransition,
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
use std::time::Instant;

pub const INSTANCE_CONTRACT_VERSION: u16 = 2;
pub const INSTANCE_FORMAT_VERSION: u16 = 2;
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
pub const MAXIMUM_INSTANCE_GRANTS: usize = 64;
pub const MAXIMUM_BLOB_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_BLOB_OBJECTS: usize = 10_000;
pub const MAXIMUM_BLOB_NAMESPACE_BYTES: u64 = 64 * 1024 * 1024;
const MAXIMUM_INSTANCE_RECORD_BYTES: usize = 4 * 1024 * 1024;

const RECORD_MAGIC: [u8; 8] = *b"LKJINS\0\x02";
const OUTCOME_MAGIC: [u8; 8] = *b"LKJOUT\0\x02";
const ATTEMPT_MAGIC: [u8; 8] = *b"LKJATT\0\x02";
const HEAD_MAGIC: [u8; 8] = *b"LKJIHEAD";
const RECORD_DOMAIN: &str = "lkjscript.instance-record.v2";
const OUTCOME_DOMAIN: &str = "lkjscript.instance-host-outcome.v2";
const ATTEMPT_DOMAIN: &str = "lkjscript.instance-host-attempt.v2";
const HEAD_DOMAIN: &str = "lkjscript.instance-head.v2";
const STATE_DOMAIN: &str = "lkjscript.instance-state.v2";
const GRANT_DOMAIN: &str = "lkjscript.host-grant.v2";
const COMMAND_DOMAIN: &str = "lkjscript.instance-command.v2";
const BLOB_CONTENT_DOMAIN: &str = "lkjscript.immutable-blob.content.v1";
const LOCK_FILE: &str = "lkjscript.instance.lock";
const APPLICATION_FILE: &str = "application.lkja";
const HEAD_FILE: &str = "HEAD";
static TEMPORARY_SERIAL: AtomicU64 = AtomicU64::new(1);

fn elapsed_nanoseconds(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn observe_execution(
    observation: &mut InstanceOperationObservation,
    transition: &StatefulTransition,
) {
    observation.lowering_nanoseconds = observation
        .lowering_nanoseconds
        .saturating_add(transition.lowering_nanoseconds);
    observation.core_verification_nanoseconds = observation
        .core_verification_nanoseconds
        .saturating_add(transition.core_verification_nanoseconds);
    observation.execution_nanoseconds = observation
        .execution_nanoseconds
        .saturating_add(transition.execute_nanoseconds);
    observation.public_value_nanoseconds = observation
        .public_value_nanoseconds
        .saturating_add(transition.public_value_nanoseconds);
}

fn observe_application_load(
    observation: &mut InstanceOperationObservation,
    application: ApplicationLoadObservation,
) {
    observation.envelope_decode_nanoseconds = observation
        .envelope_decode_nanoseconds
        .saturating_add(application.envelope_decode_nanoseconds);
    observation.canonical_reencode_nanoseconds = observation
        .canonical_reencode_nanoseconds
        .saturating_add(application.canonical_reencode_nanoseconds);
    observation.release_graph_validation_nanoseconds = observation
        .release_graph_validation_nanoseconds
        .saturating_add(application.release_graph_validation_nanoseconds);
    observation.closure_flattening_nanoseconds = observation
        .closure_flattening_nanoseconds
        .saturating_add(application.closure_flattening_nanoseconds);
    observation.release_tests_nanoseconds = observation
        .release_tests_nanoseconds
        .saturating_add(application.release_tests_nanoseconds);
    observation.release_count = observation
        .release_count
        .saturating_add(application.release_count);
    observation.flattened_semantic_items = observation
        .flattened_semantic_items
        .saturating_add(application.flattened_semantic_items);
}

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
exact_bytes!(HostGrantDigest, 32, "host grant digest");
exact_bytes!(CommandId, 32, "command ID");
exact_bytes!(BlobDigest, 32, "blob digest");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceMode {
    ValidateOnly,
    Commit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostAdapterKind {
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

/// Bounded operational observation produced by one instance-store request.
/// These timings are never durable instance authority or semantic application data.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceOperationObservation {
    pub application_read_nanoseconds: u64,
    pub application_validation_nanoseconds: u64,
    pub application_bytes: u64,
    pub envelope_decode_nanoseconds: u64,
    pub canonical_reencode_nanoseconds: u64,
    pub release_graph_validation_nanoseconds: u64,
    pub closure_flattening_nanoseconds: u64,
    pub release_tests_nanoseconds: u64,
    pub release_count: u64,
    pub flattened_semantic_items: u64,
    pub instance_open_nanoseconds: u64,
    pub record_chain_validation_nanoseconds: u64,
    pub replay_nanoseconds: u64,
    pub replay_records: u64,
    pub history_bytes: u64,
    pub transition_preparation_nanoseconds: u64,
    pub state_publication_nanoseconds: u64,
    pub grant_validation_nanoseconds: u64,
    pub adapter_preparation_nanoseconds: u64,
    pub host_action_nanoseconds: u64,
    pub outcome_publication_nanoseconds: u64,
    pub lowering_nanoseconds: u64,
    pub core_verification_nanoseconds: u64,
    pub execution_nanoseconds: u64,
    pub public_value_nanoseconds: u64,
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
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum HostGrantDescriptor {
    ApplicationActivation {
        source_directory: String,
        activation_slot: String,
    },
    ImmutableBlob {
        namespace: String,
        maximum_objects: u64,
        maximum_bytes: u64,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostGrant {
    pub version: u16,
    pub name: String,
    pub instance: InstanceId,
    pub slot: String,
    pub interface: HostInterface,
    pub adapter: HostAdapterKind,
    pub descriptor: HostGrantDescriptor,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GrantBinding {
    pub slot: String,
    pub interface: HostInterface,
    pub interface_id: HostInterfaceId,
    pub adapter: HostAdapterKind,
    pub name: String,
    pub digest: HostGrantDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceCreateRequest {
    pub version: u16,
    pub mode: InstanceMode,
    pub instance: InstanceId,
    pub initial_state: ApplicationValue,
    pub grants: Vec<HostGrant>,
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
    pub grant: HostGrant,
    pub input: HostAdapterInput,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum HostAdapterInput {
    None,
    ApplicationSource { path: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceFakeHostRequest {
    pub version: u16,
    pub instance: InstanceId,
    pub command: CommandId,
    pub grant: HostGrant,
    pub class: HostOutcomeClass,
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
    pub slot: String,
    pub interface: HostInterface,
    pub interface_id: HostInterfaceId,
    pub operation: HostOperation,
    pub request: ApplicationValue,
    pub grant: HostGrantDigest,
    pub adapter: HostAdapterKind,
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
    pub grants: Vec<GrantBinding>,
    pub published: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostExecutionReceipt {
    pub contract_version: u16,
    pub instance: InstanceId,
    pub command: CommandId,
    pub interface: HostInterfaceId,
    pub operation: HostOperation,
    pub class: HostOutcomeClass,
    pub outcome: ApplicationValue,
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
    pub host_outcome: Option<HostOutcomeClass>,
    pub host_attempted: bool,
    pub grants: Vec<GrantBinding>,
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
        class: HostOutcomeClass,
        outcome: ApplicationValue,
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
    grants: Vec<GrantBinding>,
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
    application: ApplicationDigest,
    command: CommandId,
    interface: HostInterfaceId,
    grant: HostGrantDigest,
    adapter: HostAdapterKind,
    operation: HostOperation,
    class: HostOutcomeClass,
    outcome: ApplicationValue,
    evidence: ByteString,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HostAttemptRecord {
    instance: InstanceId,
    command: CommandId,
    interface: HostInterfaceId,
    grant: HostGrantDigest,
    adapter: HostAdapterKind,
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
        self.create_observed(
            request,
            application_bytes,
            &mut InstanceOperationObservation::default(),
        )
    }

    pub fn create_observed(
        &self,
        request: &InstanceCreateRequest,
        application_bytes: &[u8],
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceCreateReceipt> {
        validate_version(request.version)?;
        validate_policy(request.policy)?;
        observation.application_bytes = observation
            .application_bytes
            .saturating_add(application_bytes.len() as u64);
        let application_validation_started = Instant::now();
        let mut application_observation = ApplicationLoadObservation::default();
        let inspection =
            application::inspect_observed(application_bytes, &mut application_observation)?;
        observe_application_load(observation, application_observation);
        let InvocationProfile::Stateful(profile) = &inspection.profile else {
            return Err(LkError::new(
                ErrorCode::RunArgumentMismatch,
                "instance creation requires a stateful application profile",
            ));
        };
        observation.application_validation_nanoseconds = observation
            .application_validation_nanoseconds
            .saturating_add(elapsed_nanoseconds(application_validation_started));
        let grant_validation_started = Instant::now();
        let grants = validate_grants(&request.grants, request.instance, &profile.imports)?;
        observation.grant_validation_nanoseconds = observation
            .grant_validation_nanoseconds
            .saturating_add(elapsed_nanoseconds(grant_validation_started));
        let transition_started = Instant::now();
        application::validate_stateful_state(application_bytes, &request.initial_state)?;
        check_value_bytes(
            &request.initial_state,
            request.policy.maximum_state_bytes,
            "initial state",
        )?;
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
            grants: grants.clone(),
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
            grants,
            published: request.mode == InstanceMode::Commit,
        };
        preflight_json(&receipt, "instance create receipt")?;
        if request.mode == InstanceMode::ValidateOnly {
            observation.transition_preparation_nanoseconds = observation
                .transition_preparation_nanoseconds
                .saturating_add(elapsed_nanoseconds(transition_started));
            return Ok(receipt);
        }
        observation.transition_preparation_nanoseconds = observation
            .transition_preparation_nanoseconds
            .saturating_add(elapsed_nanoseconds(transition_started));
        let publication_started = Instant::now();
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
        observation.state_publication_nanoseconds = observation
            .state_publication_nanoseconds
            .saturating_add(elapsed_nanoseconds(publication_started));
        Ok(receipt)
    }

    pub fn validate_event(
        &self,
        request: &InstanceEventRequest,
    ) -> Result<InstanceTransitionReceipt> {
        self.validate_event_observed(request, &mut InstanceOperationObservation::default())
    }

    pub fn validate_event_observed(
        &self,
        request: &InstanceEventRequest,
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceTransitionReceipt> {
        if request.mode != InstanceMode::ValidateOnly {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "validate-event requires validate_only mode",
            ));
        }
        self.prepare_event(request, observation)
    }

    pub fn apply_event(&self, request: &InstanceEventRequest) -> Result<InstanceTransitionReceipt> {
        self.apply_event_observed(request, &mut InstanceOperationObservation::default())
    }

    pub fn apply_event_observed(
        &self,
        request: &InstanceEventRequest,
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceTransitionReceipt> {
        if request.mode != InstanceMode::Commit {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "apply-event requires commit mode",
            ));
        }
        self.prepare_event(request, observation)
    }

    fn prepare_event(
        &self,
        request: &InstanceEventRequest,
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceTransitionReceipt> {
        validate_version(request.version)?;
        validate_event_key(request.mode, request.event_key.as_deref())?;
        let loaded = self.load_observed(request.instance, observation)?;
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
        let transition_started = Instant::now();
        let mut application_observation = ApplicationLoadObservation::default();
        let transition = application::transition_event_observed(
            &loaded.application_bytes,
            &head.state,
            &request.event,
            &mut application_observation,
        )?;
        observe_application_load(observation, application_observation);
        observation.transition_preparation_nanoseconds = observation
            .transition_preparation_nanoseconds
            .saturating_add(elapsed_nanoseconds(transition_started));
        observe_execution(observation, &transition);
        self.finish_transition(
            &loaded,
            request.mode,
            request.event_key.clone(),
            input,
            transition,
            observation,
        )
    }

    pub fn validate_resume(
        &self,
        request: &InstanceResumeRequest,
    ) -> Result<InstanceTransitionReceipt> {
        self.validate_resume_observed(request, &mut InstanceOperationObservation::default())
    }

    pub fn validate_resume_observed(
        &self,
        request: &InstanceResumeRequest,
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceTransitionReceipt> {
        if request.mode != InstanceMode::ValidateOnly {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "validate-resume requires validate_only mode",
            ));
        }
        self.prepare_resume(request, observation)
    }

    pub fn resume(&self, request: &InstanceResumeRequest) -> Result<InstanceTransitionReceipt> {
        self.resume_observed(request, &mut InstanceOperationObservation::default())
    }

    pub fn resume_observed(
        &self,
        request: &InstanceResumeRequest,
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceTransitionReceipt> {
        if request.mode != InstanceMode::Commit {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "resume requires commit mode",
            ));
        }
        self.prepare_resume(request, observation)
    }

    fn prepare_resume(
        &self,
        request: &InstanceResumeRequest,
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceTransitionReceipt> {
        validate_version(request.version)?;
        validate_event_key(request.mode, request.event_key.as_deref())?;
        let loaded = self.load_observed(request.instance, observation)?;
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
            class: outcome.class,
            outcome: outcome.outcome.clone(),
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
        let transition_started = Instant::now();
        let mut application_observation = ApplicationLoadObservation::default();
        let transition = application::transition_resume_observed(
            &loaded.application_bytes,
            &head.state,
            &outcome.outcome,
            &mut application_observation,
        )?;
        observe_application_load(observation, application_observation);
        observation.transition_preparation_nanoseconds = observation
            .transition_preparation_nanoseconds
            .saturating_add(elapsed_nanoseconds(transition_started));
        observe_execution(observation, &transition);
        self.finish_transition(
            &loaded,
            request.mode,
            request.event_key.clone(),
            input,
            transition,
            observation,
        )
    }

    pub fn execute_host(&self, request: &InstanceHostRequest) -> Result<HostExecutionReceipt> {
        self.execute_host_observed(request, &mut InstanceOperationObservation::default())
    }

    pub fn execute_host_observed(
        &self,
        request: &InstanceHostRequest,
        observation: &mut InstanceOperationObservation,
    ) -> Result<HostExecutionReceipt> {
        validate_version(request.version)?;
        let loaded = self.load_observed(request.instance, observation)?;
        reject_deleted(&loaded)?;
        let record = &loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0;
        let grant_validation_started = Instant::now();
        let command =
            validate_host_binding(record, request.instance, request.command, &request.grant)?;
        require_adapter(&request.grant, HostAdapterKind::Production)?;
        observation.grant_validation_nanoseconds = observation
            .grant_validation_nanoseconds
            .saturating_add(elapsed_nanoseconds(grant_validation_started));
        let adapter_preparation_started = Instant::now();
        if let Some(outcome) = self.read_outcome_if_present(request.instance, &command)? {
            observation.adapter_preparation_nanoseconds = observation
                .adapter_preparation_nanoseconds
                .saturating_add(elapsed_nanoseconds(adapter_preparation_started));
            return Ok(host_receipt(outcome, true));
        }
        let stateful_command = StatefulCommand {
            slot: command.slot.clone(),
            interface: command.interface,
            interface_id: command.interface_id,
            operation: command.operation,
            request: command.request.clone(),
        };
        let request_bytes = application::host_request_bytes(&stateful_command)?;
        let visibility_capable = matches!(
            command.operation,
            HostOperation::ActivateApplication | HostOperation::PutBlob
        );
        if visibility_capable && self.attempt_exists(request.instance, &command)? {
            let evidence = if command.operation == HostOperation::PutBlob {
                blob_digest_evidence(request_bytes)?
            } else {
                request_bytes.clone()
            };
            observation.adapter_preparation_nanoseconds = observation
                .adapter_preparation_nanoseconds
                .saturating_add(elapsed_nanoseconds(adapter_preparation_started));
            let outcome_publication_started = Instant::now();
            let outcome = self.make_host_outcome(
                &loaded,
                &command,
                HostOutcomeClass::OutcomeUnknown,
                evidence,
            )?;
            self.publish_outcome(&outcome)?;
            observation.outcome_publication_nanoseconds = observation
                .outcome_publication_nanoseconds
                .saturating_add(elapsed_nanoseconds(outcome_publication_started));
            return Ok(host_receipt(outcome, false));
        }
        if visibility_capable {
            self.publish_attempt(HostAttemptRecord {
                instance: request.instance,
                command: command.id,
                interface: command.interface_id,
                grant: command.grant,
                adapter: command.adapter,
            })?;
        }
        observation.adapter_preparation_nanoseconds = observation
            .adapter_preparation_nanoseconds
            .saturating_add(elapsed_nanoseconds(adapter_preparation_started));
        let host_action_started = Instant::now();
        let (class, evidence) = match command.operation {
            HostOperation::ValidateApplication => {
                validate_application_adapter(&request.grant, &request.input, request_bytes)?
            }
            HostOperation::ActivateApplication => {
                activate_application_adapter(&request.grant, &request.input, request_bytes)?
            }
            HostOperation::ReconcileActivation => {
                reconcile_activation_adapter(&request.grant, &request.input, request_bytes)?
            }
            HostOperation::PutBlob => {
                put_blob_adapter(&request.grant, &request.input, request_bytes)?
            }
            HostOperation::InspectBlob => {
                inspect_blob_adapter(&request.grant, &request.input, request_bytes)?
            }
        };
        observation.host_action_nanoseconds = observation
            .host_action_nanoseconds
            .saturating_add(elapsed_nanoseconds(host_action_started));
        let outcome_publication_started = Instant::now();
        let outcome = self.make_host_outcome(&loaded, &command, class, evidence)?;
        self.publish_outcome(&outcome)?;
        observation.outcome_publication_nanoseconds = observation
            .outcome_publication_nanoseconds
            .saturating_add(elapsed_nanoseconds(outcome_publication_started));
        Ok(host_receipt(outcome, false))
    }

    fn make_host_outcome(
        &self,
        loaded: &LoadedInstance,
        command: &PendingCommand,
        class: HostOutcomeClass,
        evidence: ByteString,
    ) -> Result<HostOutcomeRecord> {
        validate_exact_outcome_evidence(command, class, &evidence)?;
        let outcome = application::host_outcome_value(
            &loaded.application_bytes,
            &StatefulCommand {
                slot: command.slot.clone(),
                interface: command.interface,
                interface_id: command.interface_id,
                operation: command.operation,
                request: command.request.clone(),
            },
            class,
            &evidence,
        )?;
        Ok(HostOutcomeRecord {
            instance: loaded.head.instance,
            application: loaded.application,
            command: command.id,
            interface: command.interface_id,
            grant: command.grant,
            adapter: command.adapter,
            operation: command.operation,
            class,
            outcome,
            evidence,
        })
    }

    /// Records one exact scripted outcome for an instance bound to the disjoint fake adapter.
    pub fn record_fake_outcome(
        &self,
        request: &InstanceFakeHostRequest,
    ) -> Result<HostExecutionReceipt> {
        self.record_fake_outcome_observed(request, &mut InstanceOperationObservation::default())
    }

    pub fn record_fake_outcome_observed(
        &self,
        request: &InstanceFakeHostRequest,
        observation: &mut InstanceOperationObservation,
    ) -> Result<HostExecutionReceipt> {
        validate_version(request.version)?;
        let loaded = self.load_observed(request.instance, observation)?;
        reject_deleted(&loaded)?;
        let record = &loaded
            .records
            .last()
            .ok_or_else(|| corrupt("instance history is empty"))?
            .0;
        let grant_validation_started = Instant::now();
        let command =
            validate_host_binding(record, request.instance, request.command, &request.grant)?;
        require_adapter(&request.grant, HostAdapterKind::DeterministicFake)?;
        observation.grant_validation_nanoseconds = observation
            .grant_validation_nanoseconds
            .saturating_add(elapsed_nanoseconds(grant_validation_started));
        let outcome_publication_started = Instant::now();
        let proposed =
            self.make_host_outcome(&loaded, &command, request.class, request.evidence.clone())?;
        if let Some(outcome) = self.read_outcome_if_present(request.instance, &command)? {
            if outcome != proposed {
                return Err(LkError::new(
                    ErrorCode::IdempotencyConflict,
                    "fake host command already has a different exact outcome",
                ));
            }
            return Ok(host_receipt(outcome, true));
        }
        self.publish_outcome(&proposed)?;
        observation.outcome_publication_nanoseconds = observation
            .outcome_publication_nanoseconds
            .saturating_add(elapsed_nanoseconds(outcome_publication_started));
        Ok(host_receipt(proposed, false))
    }

    pub fn inspect(&self, instance: InstanceId) -> Result<InstanceInspection> {
        self.inspect_observed(instance, &mut InstanceOperationObservation::default())
    }

    pub fn inspect_observed(
        &self,
        instance: InstanceId,
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceInspection> {
        let loaded = self.load_observed(instance, observation)?;
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
            .map(|outcome| outcome.class);
        let host_attempted = if host_outcome.is_none()
            && record.command.as_ref().is_some_and(|command| {
                matches!(
                    command.operation,
                    HostOperation::ActivateApplication | HostOperation::PutBlob
                )
            }) {
            self.attempt_exists(
                instance,
                record
                    .command
                    .as_ref()
                    .ok_or_else(|| corrupt("pending command disappeared during inspection"))?,
            )?
        } else {
            false
        };
        let legal_actions = if loaded.head.deleted {
            vec!["inspect", "history"]
        } else if let Some(command) = &record.command {
            match host_outcome {
                Some(_) => vec!["inspect", "history", "validate_resume", "resume"],
                None if command.adapter == HostAdapterKind::Production => {
                    vec!["inspect", "history", "execute_host"]
                }
                None => vec!["inspect", "history", "fake_outcome"],
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
            grants: record.grants.clone(),
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
        self.history_observed(
            instance,
            start_revision,
            limit,
            &mut InstanceOperationObservation::default(),
        )
    }

    pub fn history_observed(
        &self,
        instance: InstanceId,
        start_revision: u64,
        limit: usize,
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceHistoryPage> {
        if limit == 0 || limit > MAXIMUM_INSTANCE_HISTORY_PAGE {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "instance history page limit is outside policy",
            ));
        }
        let loaded = self.load_observed(instance, observation)?;
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
        self.delete_observed(request, &mut InstanceOperationObservation::default())
    }

    pub fn delete_observed(
        &self,
        request: InstanceDeleteRequest,
        observation: &mut InstanceOperationObservation,
    ) -> Result<InstanceInspection> {
        validate_version(request.version)?;
        let loaded = self.load_observed(request.instance, observation)?;
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
        let publication_started = Instant::now();
        self.publish_head(request.instance, &head)?;
        observation.state_publication_nanoseconds = observation
            .state_publication_nanoseconds
            .saturating_add(elapsed_nanoseconds(publication_started));
        self.inspect_observed(request.instance, observation)
    }

    fn finish_transition(
        &self,
        loaded: &LoadedInstance,
        mode: InstanceMode,
        event_key: Option<String>,
        input: TransitionInput,
        transition: StatefulTransition,
        observation: &mut InstanceOperationObservation,
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
        let command = transition
            .command
            .as_ref()
            .map(|command| {
                pending_command(
                    previous.instance,
                    previous.application,
                    next_revision,
                    &previous.grants,
                    command,
                )
            })
            .transpose()?;
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
            grants: previous.grants.clone(),
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
        let publication_started = Instant::now();
        self.publish_record(previous.instance, &record, digest, &bytes)?;
        observation.state_publication_nanoseconds = observation
            .state_publication_nanoseconds
            .saturating_add(elapsed_nanoseconds(publication_started));
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

    fn load_observed(
        &self,
        instance: InstanceId,
        observation: &mut InstanceOperationObservation,
    ) -> Result<LoadedInstance> {
        let open_started = Instant::now();
        let directory = self.instance_directory(instance);
        validate_instance_directory_layout(&directory)?;
        let head_bytes = read_bounded_file(&directory.join(HEAD_FILE), 16 * 1024, "instance HEAD")?;
        let head: InstanceHead =
            decode_envelope(HEAD_MAGIC, HEAD_DOMAIN, &head_bytes, 16 * 1024)?.0;
        if head.instance != instance {
            return Err(corrupt("instance HEAD has a foreign identity"));
        }
        observation.instance_open_nanoseconds = observation
            .instance_open_nanoseconds
            .saturating_add(elapsed_nanoseconds(open_started));
        let application_read_started = Instant::now();
        let application_bytes = application::read_file(&directory.join(APPLICATION_FILE))?;
        observation.application_read_nanoseconds = observation
            .application_read_nanoseconds
            .saturating_add(elapsed_nanoseconds(application_read_started));
        observation.application_bytes = observation
            .application_bytes
            .saturating_add(application_bytes.len() as u64);
        let application_validation_started = Instant::now();
        let mut application_observation = ApplicationLoadObservation::default();
        let application_inspection =
            application::inspect_observed(&application_bytes, &mut application_observation)?;
        observe_application_load(observation, application_observation);
        observation.application_validation_nanoseconds = observation
            .application_validation_nanoseconds
            .saturating_add(elapsed_nanoseconds(application_validation_started));
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
        let record_chain_started = Instant::now();
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
        observation.record_chain_validation_nanoseconds = observation
            .record_chain_validation_nanoseconds
            .saturating_add(elapsed_nanoseconds(record_chain_started));
        observation.replay_records = observation
            .replay_records
            .saturating_add(records.len() as u64);
        observation.history_bytes = observation
            .history_bytes
            .saturating_add(records.iter().map(|entry| entry.2 as u64).sum::<u64>());
        let replay_started = Instant::now();
        validate_history(
            instance,
            application_inspection.digest,
            &application_bytes,
            &records,
        )?;
        observation.replay_nanoseconds = observation
            .replay_nanoseconds
            .saturating_add(elapsed_nanoseconds(replay_started));
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
        if outcome.instance != instance
            || outcome.command != command.id
            || outcome.interface != command.interface_id
            || outcome.grant != command.grant
            || outcome.adapter != command.adapter
            || outcome.operation != command.operation
        {
            return Err(corrupt("host outcome has a foreign command domain"));
        }
        let application_bytes =
            application::read_file(&self.instance_directory(instance).join(APPLICATION_FILE))?;
        let inspection = application::inspect(&application_bytes)?;
        if inspection.digest != outcome.application {
            return Err(corrupt("host outcome has a foreign application domain"));
        }
        validate_exact_outcome_evidence(command, outcome.class, &outcome.evidence)
            .map_err(|_| corrupt("host outcome evidence does not match its exact request"))?;
        let expected = application::host_outcome_value(
            &application_bytes,
            &StatefulCommand {
                slot: command.slot.clone(),
                interface: command.interface,
                interface_id: command.interface_id,
                operation: command.operation,
                request: command.request.clone(),
            },
            outcome.class,
            &outcome.evidence,
        )
        .map_err(|_| corrupt("host outcome is incompatible with the pending command"))?;
        if expected != outcome.outcome {
            return Err(corrupt(
                "host outcome typed value does not match its exact class mapping",
            ));
        }
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

    fn attempt_exists(&self, instance: InstanceId, command: &PendingCommand) -> Result<bool> {
        let path = self
            .instance_directory(instance)
            .join("attempts")
            .join(format!("{}.lkia", command.id));
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                let bytes = read_bounded_file(&path, 2 * 1024, "host attempt marker")?;
                let (attempt, _): (HostAttemptRecord, InstanceRecordDigest) =
                    decode_envelope(ATTEMPT_MAGIC, ATTEMPT_DOMAIN, &bytes, 1024)?;
                if attempt.instance != instance
                    || attempt.command != command.id
                    || attempt.interface != command.interface_id
                    || attempt.grant != command.grant
                    || attempt.adapter != command.adapter
                {
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
        validate_grant_bindings(&record.grants)?;
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
                        class,
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
                        let expected = application::host_outcome_value(
                            application_bytes,
                            &StatefulCommand {
                                slot: pending.slot.clone(),
                                interface: pending.interface,
                                interface_id: pending.interface_id,
                                operation: pending.operation,
                                request: pending.request.clone(),
                            },
                            *class,
                            evidence,
                        )
                        .map_err(|_| {
                            corrupt("retained host outcome is incompatible with its command")
                        })?;
                        if expected != *outcome {
                            return Err(corrupt(
                                "retained typed outcome differs from its class mapping",
                            ));
                        }
                        application::transition_resume(application_bytes, &previous.state, outcome)?
                    }
                };
                let expected_command = transition
                    .command
                    .as_ref()
                    .map(|command| {
                        pending_command(
                            instance,
                            application_digest,
                            record.revision,
                            &record.grants,
                            command,
                        )
                    })
                    .transpose()?;
                if transition.state != record.state
                    || transition.response != record.response
                    || expected_command != record.command
                {
                    return Err(corrupt(
                        "instance replay does not reproduce the retained transition",
                    ));
                }
                if record.grants != previous.grants || record.policy != previous.policy {
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
    application: ApplicationDigest,
    revision: u64,
    grants: &[GrantBinding],
    command: &StatefulCommand,
) -> Result<PendingCommand> {
    let binding = grants
        .iter()
        .find(|binding| binding.slot == command.slot)
        .ok_or_else(|| {
            LkError::new(
                ErrorCode::CapabilityDenied,
                "application command selected an import slot without a grant binding",
            )
        })?;
    if binding.interface != command.interface
        || binding.interface_id != command.interface_id
        || binding.interface_id != command.interface.identity()
    {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "application command interface does not match its exact grant binding",
        ));
    }
    let request_bytes = canonical_json(&command.request, "pending command request")?;
    let mut hasher = blake3::Hasher::new_derive_key(COMMAND_DOMAIN);
    hasher.update(&instance.as_bytes());
    hasher.update(&application.as_bytes());
    hasher.update(&revision.to_le_bytes());
    hasher.update(&binding.digest.as_bytes());
    hasher.update(&command.interface_id.as_bytes());
    hasher.update(&[command.operation as u8]);
    hasher.update(&(request_bytes.len() as u64).to_le_bytes());
    hasher.update(&request_bytes);
    Ok(PendingCommand {
        id: CommandId::from_bytes(*hasher.finalize().as_bytes()),
        slot: command.slot.clone(),
        interface: command.interface,
        interface_id: command.interface_id,
        operation: command.operation,
        request: command.request.clone(),
        grant: binding.digest,
        adapter: binding.adapter,
    })
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

fn validate_host_binding(
    record: &InstanceRecord,
    instance: InstanceId,
    command: CommandId,
    grant: &HostGrant,
) -> Result<PendingCommand> {
    validate_grant(grant, instance)?;
    let digest = grant_digest(grant)?;
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
    let binding = record
        .grants
        .iter()
        .find(|binding| binding.slot == pending.slot)
        .ok_or_else(|| corrupt("pending command grant binding is absent"))?;
    if digest != pending.grant
        || digest != binding.digest
        || grant.name != binding.name
        || grant.slot != pending.slot
        || grant.interface != pending.interface
        || grant.interface.identity() != pending.interface_id
        || grant.adapter != pending.adapter
    {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "supplied host grant does not match the pending command binding",
        ));
    }
    Ok(pending)
}

fn require_adapter(grant: &HostGrant, expected: HostAdapterKind) -> Result<()> {
    if grant.adapter == expected {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "host grant is bound to a different adapter kind",
        ))
    }
}

fn validate_exact_outcome_evidence(
    command: &PendingCommand,
    class: HostOutcomeClass,
    evidence: &ByteString,
) -> Result<()> {
    let stateful_command = StatefulCommand {
        slot: command.slot.clone(),
        interface: command.interface,
        interface_id: command.interface_id,
        operation: command.operation,
        request: command.request.clone(),
    };
    let request = application::host_request_bytes(&stateful_command)?;
    let expected = if !application::host_outcome_is_compatible(command.operation, class) {
        None
    } else if command.operation == HostOperation::PutBlob {
        Some(blob_digest_evidence(request)?)
    } else {
        Some(request.clone())
    };
    match expected {
        Some(expected) if expected == *evidence => Ok(()),
        Some(_) => Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "host outcome evidence does not match the exact request identity",
        )),
        None if evidence.is_empty() => Ok(()),
        None => Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "host outcome class requires empty evidence",
        )),
    }
}

fn host_receipt(record: HostOutcomeRecord, replayed: bool) -> HostExecutionReceipt {
    HostExecutionReceipt {
        contract_version: INSTANCE_CONTRACT_VERSION,
        instance: record.instance,
        command: record.command,
        interface: record.interface,
        operation: record.operation,
        class: record.class,
        outcome: record.outcome,
        evidence: record.evidence,
        replayed,
    }
}

fn grant_digest(grant: &HostGrant) -> Result<HostGrantDigest> {
    let bytes = canonical_json(grant, "host grant")?;
    let mut hasher = blake3::Hasher::new_derive_key(GRANT_DOMAIN);
    hasher.update(&bytes);
    Ok(HostGrantDigest::from_bytes(*hasher.finalize().as_bytes()))
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

fn validate_grants(
    grants: &[HostGrant],
    instance: InstanceId,
    imports: &[application::ApplicationImport],
) -> Result<Vec<GrantBinding>> {
    if grants.len() != imports.len() || grants.len() > MAXIMUM_INSTANCE_GRANTS {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "instance creation requires exactly one grant for every application import slot",
        ));
    }
    let mut grants = grants.to_vec();
    grants.sort_by(|left, right| left.slot.cmp(&right.slot));
    let mut bindings = Vec::with_capacity(grants.len());
    for (grant, import) in grants.iter().zip(imports) {
        validate_grant(grant, instance)?;
        if grant.slot != import.slot || grant.interface != import.interface {
            return Err(LkError::new(
                ErrorCode::CapabilityDenied,
                "host grant slot or interface does not match the exact application requirement",
            ));
        }
        bindings.push(GrantBinding {
            slot: grant.slot.clone(),
            interface: grant.interface,
            interface_id: grant.interface.identity(),
            adapter: grant.adapter,
            name: grant.name.clone(),
            digest: grant_digest(grant)?,
        });
    }
    validate_grant_bindings(&bindings)?;
    Ok(bindings)
}

fn validate_grant_bindings(bindings: &[GrantBinding]) -> Result<()> {
    if bindings.len() > MAXIMUM_INSTANCE_GRANTS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "retained grant binding count exceeds policy",
        ));
    }
    let mut prior: Option<&str> = None;
    for binding in bindings {
        validate_slot_name(&binding.slot, "grant slot")?;
        validate_slot_name(&binding.name, "grant name")?;
        if prior.is_some_and(|prior| prior >= binding.slot.as_str()) {
            return Err(corrupt(
                "retained grant bindings are not in strict canonical slot order",
            ));
        }
        if binding.interface_id != binding.interface.identity() {
            return Err(corrupt("retained grant interface identity is invalid"));
        }
        prior = Some(&binding.slot);
    }
    Ok(())
}

fn validate_grant(grant: &HostGrant, instance: InstanceId) -> Result<()> {
    validate_version(grant.version)?;
    if grant.instance != instance {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "host grant is bound to another instance",
        ));
    }
    validate_slot_name(&grant.name, "grant name")?;
    validate_slot_name(&grant.slot, "grant slot")?;
    match (&grant.interface, &grant.descriptor) {
        (
            HostInterface::ApplicationActivation,
            HostGrantDescriptor::ApplicationActivation {
                source_directory,
                activation_slot,
            },
        ) => {
            let source = Path::new(source_directory);
            let slot = Path::new(activation_slot);
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
        }
        (
            HostInterface::ImmutableBlob,
            HostGrantDescriptor::ImmutableBlob {
                namespace,
                maximum_objects,
                maximum_bytes,
            },
        ) => {
            if *maximum_objects == 0
                || *maximum_objects > MAXIMUM_BLOB_OBJECTS as u64
                || *maximum_bytes == 0
                || *maximum_bytes > MAXIMUM_BLOB_NAMESPACE_BYTES
            {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "immutable-blob grant limits are outside global bounds",
                ));
            }
            let namespace = Path::new(namespace);
            validate_absolute_path(namespace, "immutable blob namespace")?;
            validate_parent_chain(namespace, "immutable blob namespace")?;
            let metadata = fs::symlink_metadata(namespace).map_err(LkError::from)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LkError::new(
                    ErrorCode::CapabilityDenied,
                    "immutable-blob namespace is not a regular directory",
                ));
            }
            ensure_private_directory(namespace)?;
        }
        _ => {
            return Err(LkError::new(
                ErrorCode::CapabilityDenied,
                "host grant descriptor belongs to another interface",
            ));
        }
    }
    Ok(())
}

fn validate_slot_name(value: &str, label: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= MAXIMUM_GRANT_NAME_BYTES
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .skip(1)
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} is not canonical"),
        ))
    }
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

fn activation_descriptor(grant: &HostGrant) -> Result<(&Path, &Path)> {
    match &grant.descriptor {
        HostGrantDescriptor::ApplicationActivation {
            source_directory,
            activation_slot,
        } if grant.interface == HostInterface::ApplicationActivation => {
            Ok((Path::new(source_directory), Path::new(activation_slot)))
        }
        _ => Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "activation operation requires an exact activation grant descriptor",
        )),
    }
}

fn application_source<'a>(input: &'a HostAdapterInput, label: &str) -> Result<&'a Path> {
    match input {
        HostAdapterInput::ApplicationSource { path } => Ok(Path::new(path)),
        HostAdapterInput::None => Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} requires one explicit application source"),
        )),
    }
}

fn require_no_adapter_input(input: &HostAdapterInput) -> Result<()> {
    if *input == HostAdapterInput::None {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "this host operation accepts no adapter input",
        ))
    }
}

fn exact_application_request(bytes: &ByteString) -> Result<ApplicationDigest> {
    let digest = <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            "activation request does not carry one exact application digest",
        )
    })?;
    Ok(ApplicationDigest::from_bytes(digest))
}

fn validate_application_adapter(
    grant: &HostGrant,
    input: &HostAdapterInput,
    request: &ByteString,
) -> Result<(HostOutcomeClass, ByteString)> {
    let target = exact_application_request(request)?;
    let (source_directory, _) = activation_descriptor(grant)?;
    let source = application_source(input, "application validation")?;
    validate_source_path(source, source_directory)?;
    match application::read_file(source).and_then(|bytes| application::inspect(&bytes)) {
        Ok(inspection) if inspection.digest == target => Ok((
            HostOutcomeClass::Succeeded,
            ByteString::from_slice(&target.as_bytes()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "validation evidence exceeds policy",
                )
            })?,
        )),
        _ => Ok((
            HostOutcomeClass::KnownFailureBeforeVisibility,
            ByteString::from_slice(&target.as_bytes()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "validation evidence exceeds policy",
                )
            })?,
        )),
    }
}

fn activate_application_adapter(
    grant: &HostGrant,
    input: &HostAdapterInput,
    request: &ByteString,
) -> Result<(HostOutcomeClass, ByteString)> {
    let target = exact_application_request(request)?;
    let (source_directory, slot) = activation_descriptor(grant)?;
    let source = application_source(input, "application activation")?;
    validate_source_path(source, source_directory)?;
    let source_bytes = application::read_file(source)?;
    let source_inspection = application::inspect(&source_bytes)?;
    if source_inspection.digest != target {
        return Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "activation source does not match the exact requested application digest",
        ));
    }
    match activate_slot(slot, &source_bytes) {
        Ok(()) => Ok((
            HostOutcomeClass::Succeeded,
            ByteString::from_slice(&target.as_bytes()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "activation evidence exceeds policy",
                )
            })?,
        )),
        Err(error) if error.code == ErrorCode::ArtifactPublicationOutcomeUnknown => Ok((
            HostOutcomeClass::OutcomeUnknown,
            ByteString::from_slice(&target.as_bytes()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "activation evidence exceeds policy",
                )
            })?,
        )),
        Err(_) => Ok((
            HostOutcomeClass::KnownFailureBeforeVisibility,
            ByteString::from_slice(&target.as_bytes()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "activation evidence exceeds policy",
                )
            })?,
        )),
    }
}

fn reconcile_activation_adapter(
    grant: &HostGrant,
    input: &HostAdapterInput,
    request: &ByteString,
) -> Result<(HostOutcomeClass, ByteString)> {
    require_no_adapter_input(input)?;
    let target = exact_application_request(request)?;
    let (_, slot) = activation_descriptor(grant)?;
    match fs::symlink_metadata(slot) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((
            HostOutcomeClass::ReconciliationAbsent,
            ByteString::from_slice(&target.as_bytes()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "reconciliation evidence exceeds policy",
                )
            })?,
        )),
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            match application::read_file(slot).and_then(|bytes| application::inspect(&bytes)) {
                Ok(inspection) if inspection.digest == target => Ok((
                    HostOutcomeClass::ReconciliationPresent,
                    ByteString::from_slice(&target.as_bytes()).map_err(|_| {
                        LkError::new(
                            ErrorCode::PolicyExceeded,
                            "reconciliation evidence exceeds policy",
                        )
                    })?,
                )),
                _ => Ok((
                    HostOutcomeClass::ReconciliationIndeterminate,
                    ByteString::from_slice(&target.as_bytes()).map_err(|_| {
                        LkError::new(
                            ErrorCode::PolicyExceeded,
                            "reconciliation evidence exceeds policy",
                        )
                    })?,
                )),
            }
        }
        _ => Ok((
            HostOutcomeClass::ReconciliationIndeterminate,
            ByteString::from_slice(&target.as_bytes()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "reconciliation evidence exceeds policy",
                )
            })?,
        )),
    }
}

fn blob_descriptor(grant: &HostGrant) -> Result<(&Path, u64, u64)> {
    match &grant.descriptor {
        HostGrantDescriptor::ImmutableBlob {
            namespace,
            maximum_objects,
            maximum_bytes,
        } if grant.interface == HostInterface::ImmutableBlob => {
            Ok((Path::new(namespace), *maximum_objects, *maximum_bytes))
        }
        _ => Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "blob operation requires an exact immutable-blob grant descriptor",
        )),
    }
}

fn blob_digest(bytes: &[u8]) -> BlobDigest {
    let mut hasher = blake3::Hasher::new_derive_key(BLOB_CONTENT_DOMAIN);
    hasher.update(bytes);
    BlobDigest::from_bytes(*hasher.finalize().as_bytes())
}

fn blob_digest_evidence(bytes: &ByteString) -> Result<ByteString> {
    ByteString::from_slice(&blob_digest(bytes.as_slice()).as_bytes()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "blob digest evidence exceeds policy",
        )
    })
}

fn blob_path(namespace: &Path, digest: BlobDigest) -> PathBuf {
    namespace.join(format!("{digest}.lkjb"))
}

fn blob_namespace_usage(namespace: &Path) -> Result<(u64, u64)> {
    let mut objects = 0_u64;
    let mut bytes = 0_u64;
    for entry in fs::read_dir(namespace)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| corrupt("blob namespace contains a non-UTF-8 object name"))?;
        let Some(digest) = name.strip_suffix(".lkjb") else {
            return Err(corrupt("blob namespace contains a foreign object"));
        };
        digest
            .parse::<BlobDigest>()
            .map_err(|_| corrupt("blob namespace object name is not canonical"))?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(corrupt("blob namespace object is not a regular file"));
        }
        objects = objects
            .checked_add(1)
            .ok_or_else(|| LkError::new(ErrorCode::PolicyExceeded, "blob count overflows"))?;
        bytes = bytes
            .checked_add(metadata.len())
            .ok_or_else(|| LkError::new(ErrorCode::PolicyExceeded, "blob bytes overflow"))?;
    }
    Ok((objects, bytes))
}

fn read_blob(path: &Path, expected: BlobDigest) -> Result<Vec<u8>> {
    let bytes = read_bounded_file(path, MAXIMUM_BLOB_BYTES, "immutable blob")?;
    if blob_digest(&bytes) != expected {
        return Err(corrupt(
            "immutable blob content does not match its exact object name",
        ));
    }
    Ok(bytes)
}

fn put_blob_adapter(
    grant: &HostGrant,
    input: &HostAdapterInput,
    request: &ByteString,
) -> Result<(HostOutcomeClass, ByteString)> {
    require_no_adapter_input(input)?;
    if request.len() > MAXIMUM_BLOB_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "immutable blob exceeds content byte policy",
        ));
    }
    let (namespace, maximum_objects, maximum_bytes) = blob_descriptor(grant)?;
    let digest = blob_digest(request.as_slice());
    let evidence = ByteString::from_slice(&digest.as_bytes()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "blob digest evidence exceeds policy",
        )
    })?;
    let path = blob_path(namespace, digest);
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(corrupt("immutable blob object is not a regular file"));
            }
            if read_blob(&path, digest)? != request.as_slice() {
                return Err(corrupt(
                    "immutable blob identity conflicts with existing bytes",
                ));
            }
            return Ok((HostOutcomeClass::AlreadyPresent, evidence));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let (objects, retained_bytes) = blob_namespace_usage(namespace)?;
    if objects.saturating_add(1) > maximum_objects
        || retained_bytes.saturating_add(request.len() as u64) > maximum_bytes
    {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "immutable-blob grant capacity would be exceeded",
        ));
    }
    match publish_immutable(namespace, &path, request.as_slice(), ".blob-") {
        Ok(()) => Ok((HostOutcomeClass::Succeeded, evidence)),
        Err(error) if error.code == ErrorCode::ArtifactPublicationOutcomeUnknown => {
            Ok((HostOutcomeClass::OutcomeUnknown, evidence))
        }
        Err(_) => Ok((HostOutcomeClass::KnownFailureBeforeVisibility, evidence)),
    }
}

fn inspect_blob_adapter(
    grant: &HostGrant,
    input: &HostAdapterInput,
    request: &ByteString,
) -> Result<(HostOutcomeClass, ByteString)> {
    require_no_adapter_input(input)?;
    let (namespace, _, _) = blob_descriptor(grant)?;
    let digest = <[u8; 32]>::try_from(request.as_slice())
        .map(BlobDigest::from_bytes)
        .map_err(|_| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "blob inspection requires one exact 32-byte digest",
            )
        })?;
    let path = blob_path(namespace, digest);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((
            HostOutcomeClass::ReconciliationAbsent,
            ByteString::from_slice(&digest.as_bytes()).map_err(|_| {
                LkError::new(ErrorCode::PolicyExceeded, "blob evidence exceeds policy")
            })?,
        )),
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            read_blob(&path, digest)?;
            Ok((
                HostOutcomeClass::ReconciliationPresent,
                ByteString::from_slice(&digest.as_bytes()).map_err(|_| {
                    LkError::new(ErrorCode::PolicyExceeded, "blob evidence exceeds policy")
                })?,
            ))
        }
        _ => Err(corrupt(
            "immutable blob object has an invalid filesystem type",
        )),
    }
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
