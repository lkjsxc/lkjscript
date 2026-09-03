//! Strict standalone Artifact 14 deployment and normalized resident execution.

use super::compiler::{MAXIMUM_ARTIFACT_BUNDLE_BYTES, load_artifact};
use super::configuration::{
    ConfigurationObservation, ConfigurationStore, ConfigurationValue, MAXIMUM_CONFIGURATION_FIELDS,
    MAXIMUM_CONFIGURATION_VALUE_BYTES,
};
use super::data::{
    DataLimits, MAXIMUM_DATA_KEY_BYTES, MAXIMUM_DATA_KEY_PARTS, MAXIMUM_DATA_LIVE_TRANSACTIONS,
    MAXIMUM_DATA_SCAN_BYTES, MAXIMUM_DATA_SCAN_ITEMS, MAXIMUM_DATA_SCAN_WORK,
    MAXIMUM_DATA_SPACE_NAME_BYTES, MAXIMUM_DATA_TRANSACTION_BYTES,
    MAXIMUM_DATA_TRANSACTION_MUTATIONS, MAXIMUM_DATA_VALUE_BYTES,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::RunPolicy;
use super::execution::normalized::{
    NormalizedAdapterDescriptor, NormalizedDeploymentGrant, NormalizedDeploymentResourcePolicy,
    NormalizedGrantAuthorityRevision, NormalizedGrantLimit, NormalizedHttpApplication,
    NormalizedPreparedDeployment, NormalizedProgram, NormalizedResidentDeployment,
    NormalizedRunPolicy, NormalizedSessionApplication, NormalizedSharingDomain,
    NormalizedWorkerApplication,
};
use super::http::{
    HttpDispatchObservation, HttpLimits, HttpRequest, HttpResponse, HttpServerReceipt,
    MAXIMUM_HTTP_BODY_BYTES, MAXIMUM_HTTP_HEADER_BYTES, MAXIMUM_HTTP_HEADERS,
};
use super::http_client::{
    HttpClientAddressPolicy, HttpClientLimits, HttpClientTrust,
    MAXIMUM_HTTP_CLIENT_CONCURRENT_REQUESTS, MAXIMUM_HTTP_CLIENT_DNS_RESULTS,
    MAXIMUM_HTTP_CLIENT_ENDPOINT_BYTES, MAXIMUM_HTTP_CLIENT_MILLISECONDS,
    MAXIMUM_HTTP_CLIENT_REQUEST_HEADER_BYTES, MAXIMUM_HTTP_CLIENT_REQUEST_HEADERS,
    MAXIMUM_HTTP_CLIENT_RESPONSE_BODY_BYTES, MAXIMUM_HTTP_CLIENT_RESPONSE_HEADER_BYTES,
    MAXIMUM_HTTP_CLIENT_RESPONSE_HEADERS, validate_http_client_descriptor,
};
use super::kernel::Name;
use super::object::{MAXIMUM_OBJECT_BYTES, MAXIMUM_OBJECT_KEY_BYTES, ObjectLimits};
use super::package::RunnerKind;
use super::queue::{
    MAXIMUM_QUEUE_ATTEMPTS, MAXIMUM_QUEUE_LEASE_MILLISECONDS, MAXIMUM_QUEUE_PAYLOAD_BYTES,
    QueueLimits,
};
use super::runtime::{
    MAXIMUM_CONCURRENT_TASKS, MAXIMUM_OPERATIONAL_MILLISECONDS, MAXIMUM_QUEUED_TASKS,
    ResidentLimits, ResidentObservation, ShutdownReceipt,
};
use super::secrets::{EnvironmentSecretBinding, MAXIMUM_SECRET_BYTES, SecretCatalog};
use super::security::PasswordHashPolicy;
use super::session::{
    MAXIMUM_ACTIVE_SESSIONS, MAXIMUM_PENDING_HANDSHAKES, MAXIMUM_PROCESS_SESSION_BUFFER_BYTES,
    MAXIMUM_SESSION_FRAME_BYTES, MAXIMUM_SESSION_GRACE_MILLISECONDS, MAXIMUM_SESSION_HEADER_BYTES,
    MAXIMUM_SESSION_HEADERS, MAXIMUM_SESSION_INTERVAL_MILLISECONDS,
    MAXIMUM_SESSION_LIFETIME_MILLISECONDS, MAXIMUM_SESSION_MAILBOX_BYTES,
    MAXIMUM_SESSION_MAILBOX_ITEMS, MAXIMUM_SESSION_MESSAGE_BYTES, MAXIMUM_SESSION_STATE_BYTES,
    MAXIMUM_SESSION_STATE_NODES, MAXIMUM_SESSION_TRANSITION_BYTES,
    MAXIMUM_SESSION_TRANSITION_MESSAGES, SessionLimits, SessionObservation, SessionServerReceipt,
};
use super::stream::{
    MAXIMUM_LIVE_STREAMS, MAXIMUM_STREAM_BUFFERED_CHUNKS, MAXIMUM_STREAM_CHUNK_BYTES, StreamLimits,
};
use super::worker::{
    MAXIMUM_IDLE_WAIT_MILLISECONDS, MAXIMUM_RESIDENT_WORKERS, WorkerLimits, WorkerReceipt,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Handle;

pub const DEPLOYMENT_CONTRACT_VERSION: u16 = 4;
pub const MAXIMUM_DEPLOYMENT_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_DEPLOYMENT_GRANTS: usize = 1_024;
pub(crate) const STARTER_HTTP_DESCRIPTOR_PATH: &str = "service.deployment.json";
pub(crate) const STARTER_HTTP_ARTIFACT_PATH: &str = "generated/application.lkja";
pub(crate) const STARTER_HTTP_ARTIFACT_DIRECTORY: &str = "generated";
pub(crate) const STARTER_HTTP_TARGET: &str = "serve";
pub(crate) const STARTER_HTTP_LISTENER: &str = "127.0.0.1:0";

const fn deployment_contract_version() -> u16 {
    DEPLOYMENT_CONTRACT_VERSION
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeploymentAuthorityRevision([u8; 32]);

impl DeploymentAuthorityRevision {
    fn generate() -> Result<Self, Diagnostic> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "deployment_authority_entropy",
                "operating-system entropy is unavailable for starter deployment authority",
            )
        })?;
        if bytes == [0; 32] {
            bytes[31] = 1;
        }
        Ok(Self(bytes))
    }

    fn encode(self) -> String {
        super::semantic_id::encode_hex(&self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentDescriptor {
    #[serde(skip, default = "deployment_contract_version")]
    pub contract_version: u16,
    pub artifact: String,
    pub target: String,
    pub listen: Option<String>,
    pub runtime: ResidentLimits,
    pub execution: RunPolicy,
    pub http: Option<HttpLimits>,
    pub session: Option<SessionLimits>,
    pub worker: Option<WorkerLimits>,
    pub streams: StreamLimits,
    pub configuration: BTreeMap<String, ConfigurationValue>,
    pub secrets: Vec<EnvironmentSecretBinding>,
    pub grants: Vec<DeploymentGrant>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentGrant {
    pub requirement: String,
    pub sharing_domain: String,
    pub authority_revision: String,
    pub adapter: AdapterDescriptor,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub enum AdapterDescriptor {
    Configuration,
    WallClock,
    SecureRandom,
    Identifier,
    PasswordHash {
        policy: PasswordHashPolicy,
    },
    SecretVerifier {
        secret: String,
        maximum_candidate_bytes: usize,
    },
    ByteStream,
    HttpClient {
        endpoint: String,
        address_policy: HttpClientAddressPolicy,
        trust: HttpClientTrust,
        limits: HttpClientLimits,
    },
    Data {
        root: String,
        namespace: String,
        limits: DataLimits,
    },
    ObjectMemory {
        prefix: String,
        limits: ObjectLimits,
    },
    ObjectLocal {
        root: String,
        prefix: String,
        limits: ObjectLimits,
    },
    ObjectS3 {
        endpoint: String,
        region: String,
        bucket: String,
        prefix: String,
        allow_http: bool,
        path_style: bool,
        access_key_secret: String,
        secret_key_secret: String,
        limits: ObjectLimits,
    },
    DurableQueueData {
        root: String,
        namespace: String,
        data_limits: DataLimits,
        limits: QueueLimits,
    },
}

/// One field in the executable-owned strict deployment descriptor inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeploymentSchemaField {
    pub(crate) path: &'static str,
    pub(crate) required: bool,
    pub(crate) scalar: &'static str,
    pub(crate) minimum: Option<u64>,
    pub(crate) maximum: Option<u64>,
    pub(crate) secret_name: bool,
    pub(crate) nested: Option<&'static str>,
}

/// One closed adapter variant and its exact strict-JSON fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeploymentAdapterSchema {
    pub(crate) kind: &'static str,
    pub(crate) fields: &'static [DeploymentSchemaField],
}

const fn schema_field(
    path: &'static str,
    scalar: &'static str,
    minimum: Option<u64>,
    maximum: Option<u64>,
    secret_name: bool,
    nested: Option<&'static str>,
) -> DeploymentSchemaField {
    DeploymentSchemaField {
        path,
        required: true,
        scalar,
        minimum,
        maximum,
        secret_name,
        nested,
    }
}

const fn optional_schema_field(
    path: &'static str,
    scalar: &'static str,
    minimum: Option<u64>,
    maximum: Option<u64>,
    secret_name: bool,
    nested: Option<&'static str>,
) -> DeploymentSchemaField {
    DeploymentSchemaField {
        path,
        required: false,
        scalar,
        minimum,
        maximum,
        secret_name,
        nested,
    }
}

pub(crate) const DEPLOYMENT_SCHEMA_FIELDS: &[DeploymentSchemaField] = &[
    schema_field(
        "deployment.artifact",
        "relative-path",
        Some(1),
        Some(4096),
        false,
        None,
    ),
    schema_field("deployment.target", "name", Some(1), Some(128), false, None),
    optional_schema_field(
        "deployment.listen",
        "null|string",
        Some(1),
        Some(512),
        false,
        None,
    ),
    schema_field(
        "deployment.runtime",
        "object",
        None,
        None,
        false,
        Some("runtime"),
    ),
    schema_field(
        "deployment.execution",
        "object",
        None,
        None,
        false,
        Some("execution"),
    ),
    optional_schema_field(
        "deployment.http",
        "null|object",
        None,
        None,
        false,
        Some("http"),
    ),
    optional_schema_field(
        "deployment.session",
        "null|object",
        None,
        None,
        false,
        Some("session"),
    ),
    optional_schema_field(
        "deployment.worker",
        "null|object",
        None,
        None,
        false,
        Some("worker"),
    ),
    schema_field(
        "deployment.streams",
        "object",
        None,
        None,
        false,
        Some("streams"),
    ),
    schema_field(
        "deployment.configuration",
        "map",
        Some(0),
        Some(MAXIMUM_CONFIGURATION_FIELDS as u64),
        false,
        Some("configuration-value"),
    ),
    schema_field(
        "deployment.secrets",
        "array",
        Some(0),
        Some(MAXIMUM_DEPLOYMENT_GRANTS as u64),
        false,
        Some("secret-binding"),
    ),
    schema_field(
        "deployment.grants",
        "array",
        Some(0),
        Some(MAXIMUM_DEPLOYMENT_GRANTS as u64),
        false,
        Some("grant"),
    ),
    schema_field(
        "runtime.maximum_concurrent_tasks",
        "usize",
        Some(1),
        Some(MAXIMUM_CONCURRENT_TASKS as u64),
        false,
        None,
    ),
    schema_field(
        "runtime.maximum_queued_tasks",
        "usize",
        Some(0),
        Some(MAXIMUM_QUEUED_TASKS as u64),
        false,
        None,
    ),
    schema_field(
        "runtime.request_deadline_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_OPERATIONAL_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "runtime.shutdown_grace_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_OPERATIONAL_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "runtime.cancellation_grace_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_OPERATIONAL_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "execution.instruction_fuel",
        "u64",
        Some(1),
        Some(u64::MAX),
        false,
        None,
    ),
    schema_field(
        "execution.maximum_call_depth",
        "usize",
        Some(1),
        Some(usize::MAX as u64),
        false,
        None,
    ),
    schema_field(
        "execution.maximum_value_stack",
        "usize",
        Some(1),
        Some(usize::MAX as u64),
        false,
        None,
    ),
    schema_field(
        "http.maximum_request_body_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_BODY_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "http.maximum_response_body_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_BODY_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "http.maximum_header_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_HEADER_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "http.maximum_headers",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_HEADERS as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_active_sessions",
        "usize",
        Some(1),
        Some(MAXIMUM_ACTIVE_SESSIONS as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_pending_handshakes",
        "usize",
        Some(1),
        Some(MAXIMUM_PENDING_HANDSHAKES as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_message_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_MESSAGE_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_frame_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_FRAME_BYTES as u64),
        false,
        Some("at-most-session.maximum_message_bytes"),
    ),
    schema_field(
        "session.maximum_header_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_HEADER_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_headers",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_HEADERS as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_inbound_mailbox_items",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_MAILBOX_ITEMS as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_inbound_mailbox_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_MAILBOX_BYTES as u64),
        false,
        Some("at-least-session.maximum_message_bytes"),
    ),
    schema_field(
        "session.maximum_outbound_mailbox_items",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_MAILBOX_ITEMS as u64),
        false,
        Some("at-least-session.maximum_transition_messages"),
    ),
    schema_field(
        "session.maximum_outbound_mailbox_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_MAILBOX_BYTES as u64),
        false,
        Some("at-least-session.maximum_transition_bytes"),
    ),
    schema_field(
        "session.maximum_state_nodes",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_STATE_NODES as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_state_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_STATE_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_transition_messages",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_TRANSITION_MESSAGES as u64),
        false,
        None,
    ),
    schema_field(
        "session.maximum_transition_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_SESSION_TRANSITION_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "session.tick_interval_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_SESSION_INTERVAL_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "session.idle_timeout_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_SESSION_LIFETIME_MILLISECONDS),
        false,
        Some("at-most-session.maximum_lifetime_milliseconds"),
    ),
    schema_field(
        "session.maximum_lifetime_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_SESSION_LIFETIME_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "session.close_grace_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_SESSION_GRACE_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "session.cancellation_grace_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_SESSION_GRACE_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "session.maximum_process_buffer_bytes",
        "u64",
        Some(1),
        Some(MAXIMUM_PROCESS_SESSION_BUFFER_BYTES),
        false,
        Some("at-least-one-session-reservation"),
    ),
    schema_field(
        "worker.maximum_workers",
        "usize",
        Some(1),
        Some(MAXIMUM_RESIDENT_WORKERS as u64),
        false,
        Some("at-most-runtime.maximum_concurrent_tasks"),
    ),
    schema_field(
        "worker.idle_wait_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_IDLE_WAIT_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "streams.maximum_chunk_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_STREAM_CHUNK_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "streams.maximum_buffered_chunks",
        "usize",
        Some(1),
        Some(MAXIMUM_STREAM_BUFFERED_CHUNKS as u64),
        false,
        None,
    ),
    schema_field(
        "streams.maximum_total_bytes",
        "u64",
        Some(1),
        Some(u64::MAX),
        false,
        None,
    ),
    schema_field(
        "streams.maximum_live_streams",
        "usize",
        Some(1),
        Some(MAXIMUM_LIVE_STREAMS as u64),
        false,
        None,
    ),
    schema_field(
        "configuration-value.kind",
        "enum:text|i64|bool",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "configuration-value.value",
        "string|i64|bool",
        Some(0),
        Some(MAXIMUM_CONFIGURATION_VALUE_BYTES as u64),
        false,
        Some("kind-selected"),
    ),
    schema_field(
        "configuration-entry.name",
        "configuration-name",
        Some(1),
        Some(256),
        false,
        None,
    ),
    schema_field(
        "secret-binding.name",
        "name",
        Some(1),
        Some(256),
        true,
        None,
    ),
    schema_field(
        "secret-binding.variable",
        "environment-name",
        Some(1),
        Some(256),
        false,
        None,
    ),
    schema_field("grant.requirement", "name", Some(1), Some(128), false, None),
    schema_field(
        "grant.sharing_domain",
        "name",
        Some(1),
        Some(128),
        false,
        None,
    ),
    schema_field(
        "grant.authority_revision",
        "lowercase-hex",
        Some(64),
        Some(64),
        false,
        None,
    ),
    schema_field(
        "grant.adapter",
        "tagged-object",
        None,
        None,
        false,
        Some("adapter"),
    ),
    schema_field(
        "password-policy.memory_kibibytes",
        "u32",
        Some(8),
        Some(1_048_576),
        false,
        Some("at-least-8-times-lanes"),
    ),
    schema_field(
        "password-policy.iterations",
        "u32",
        Some(1),
        Some(32),
        false,
        None,
    ),
    schema_field(
        "password-policy.lanes",
        "u32",
        Some(1),
        Some(16),
        false,
        None,
    ),
    schema_field(
        "password-policy.output_bytes",
        "usize",
        Some(16),
        Some(64),
        false,
        None,
    ),
    schema_field(
        "object-limits.maximum_object_bytes",
        "u64",
        Some(1),
        Some(MAXIMUM_OBJECT_BYTES),
        false,
        None,
    ),
    schema_field(
        "object-limits.maximum_whole_read_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_OBJECT_BYTES),
        false,
        Some("at-most-maximum_object_bytes"),
    ),
    schema_field(
        "queue-limits.maximum_payload_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_QUEUE_PAYLOAD_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "queue-limits.maximum_result_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_QUEUE_PAYLOAD_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "queue-limits.maximum_lease_milliseconds",
        "i64",
        Some(1),
        Some(MAXIMUM_QUEUE_LEASE_MILLISECONDS as u64),
        false,
        None,
    ),
    schema_field(
        "queue-limits.maximum_attempts",
        "u32",
        Some(1),
        Some(MAXIMUM_QUEUE_ATTEMPTS as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_space_name_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_SPACE_NAME_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_key_parts",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_KEY_PARTS as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_key_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_KEY_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_value_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_VALUE_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_transaction_mutations",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_TRANSACTION_MUTATIONS as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_transaction_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_TRANSACTION_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_scan_items",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_SCAN_ITEMS as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_scan_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_SCAN_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_scan_work",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_SCAN_WORK as u64),
        false,
        None,
    ),
    schema_field(
        "data-limits.maximum_live_transactions",
        "usize",
        Some(1),
        Some(MAXIMUM_DATA_LIVE_TRANSACTIONS as u64),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.maximum_request_headers",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_REQUEST_HEADERS as u64),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.maximum_request_header_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_REQUEST_HEADER_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.maximum_response_headers",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_RESPONSE_HEADERS as u64),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.maximum_response_header_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_RESPONSE_HEADER_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.maximum_response_body_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_RESPONSE_BODY_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.maximum_dns_results",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_DNS_RESULTS as u64),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.maximum_concurrent_requests",
        "usize",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_CONCURRENT_REQUESTS as u64),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.connection_timeout_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.total_timeout_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_MILLISECONDS),
        false,
        None,
    ),
    schema_field(
        "http-client-limits.cleanup_timeout_milliseconds",
        "u64",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_MILLISECONDS),
        false,
        None,
    ),
];

const ADAPTER_CONFIGURATION_FIELDS: &[DeploymentSchemaField] = &[schema_field(
    "adapter.configuration.kind",
    "literal:configuration",
    None,
    None,
    false,
    None,
)];
const ADAPTER_WALL_CLOCK_FIELDS: &[DeploymentSchemaField] = &[schema_field(
    "adapter.wall_clock.kind",
    "literal:wall_clock",
    None,
    None,
    false,
    None,
)];
const ADAPTER_SECURE_RANDOM_FIELDS: &[DeploymentSchemaField] = &[schema_field(
    "adapter.secure_random.kind",
    "literal:secure_random",
    None,
    None,
    false,
    None,
)];
const ADAPTER_IDENTIFIER_FIELDS: &[DeploymentSchemaField] = &[schema_field(
    "adapter.identifier.kind",
    "literal:identifier",
    None,
    None,
    false,
    None,
)];
const ADAPTER_PASSWORD_HASH_FIELDS: &[DeploymentSchemaField] = &[
    schema_field(
        "adapter.password_hash.kind",
        "literal:password_hash",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.password_hash.policy",
        "object",
        None,
        None,
        false,
        Some("password-policy"),
    ),
];
const ADAPTER_SECRET_VERIFIER_FIELDS: &[DeploymentSchemaField] = &[
    schema_field(
        "adapter.secret_verifier.kind",
        "literal:secret_verifier",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.secret_verifier.secret",
        "name",
        Some(1),
        Some(128),
        true,
        None,
    ),
    schema_field(
        "adapter.secret_verifier.maximum_candidate_bytes",
        "usize",
        Some(1),
        Some(MAXIMUM_SECRET_BYTES as u64),
        false,
        None,
    ),
];
const ADAPTER_BYTE_STREAM_FIELDS: &[DeploymentSchemaField] = &[schema_field(
    "adapter.byte_stream.kind",
    "literal:byte_stream",
    None,
    None,
    false,
    None,
)];
const ADAPTER_HTTP_CLIENT_FIELDS: &[DeploymentSchemaField] = &[
    schema_field(
        "adapter.http_client.kind",
        "literal:http_client",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.http_client.endpoint",
        "canonical-http-endpoint",
        Some(1),
        Some(MAXIMUM_HTTP_CLIENT_ENDPOINT_BYTES as u64),
        false,
        None,
    ),
    schema_field(
        "adapter.http_client.address_policy",
        "enum:public_only|loopback_only",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.http_client.trust",
        "tagged-object",
        None,
        None,
        false,
        Some("http-client-trust"),
    ),
    schema_field(
        "adapter.http_client.limits",
        "object",
        None,
        None,
        false,
        Some("http-client-limits"),
    ),
];
const ADAPTER_DATA_FIELDS: &[DeploymentSchemaField] = &[
    schema_field("adapter.data.kind", "literal:data", None, None, false, None),
    schema_field(
        "adapter.data.root",
        "relative-path",
        Some(1),
        Some(4096),
        false,
        None,
    ),
    schema_field(
        "adapter.data.namespace",
        "deployment-token",
        Some(1),
        Some(128),
        false,
        None,
    ),
    schema_field(
        "adapter.data.limits",
        "object",
        None,
        None,
        false,
        Some("data-limits"),
    ),
];
const ADAPTER_OBJECT_MEMORY_FIELDS: &[DeploymentSchemaField] = &[
    schema_field(
        "adapter.object_memory.kind",
        "literal:object_memory",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.object_memory.prefix",
        "object-prefix",
        Some(0),
        Some(1024),
        false,
        None,
    ),
    schema_field(
        "adapter.object_memory.limits",
        "object",
        None,
        None,
        false,
        Some("object-limits"),
    ),
];
const ADAPTER_OBJECT_LOCAL_FIELDS: &[DeploymentSchemaField] = &[
    schema_field(
        "adapter.object_local.kind",
        "literal:object_local",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.object_local.root",
        "relative-path",
        Some(1),
        Some(4096),
        false,
        None,
    ),
    schema_field(
        "adapter.object_local.prefix",
        "object-prefix",
        Some(0),
        Some(1024),
        false,
        None,
    ),
    schema_field(
        "adapter.object_local.limits",
        "object",
        None,
        None,
        false,
        Some("object-limits"),
    ),
];
const ADAPTER_OBJECT_S3_FIELDS: &[DeploymentSchemaField] = &[
    schema_field(
        "adapter.object_s3.kind",
        "literal:object_s3",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.object_s3.endpoint",
        "string",
        Some(1),
        Some(4096),
        false,
        None,
    ),
    schema_field(
        "adapter.object_s3.region",
        "deployment-token",
        Some(1),
        Some(255),
        false,
        None,
    ),
    schema_field(
        "adapter.object_s3.bucket",
        "deployment-token",
        Some(1),
        Some(255),
        false,
        None,
    ),
    schema_field(
        "adapter.object_s3.prefix",
        "object-prefix",
        Some(0),
        Some(1024),
        false,
        None,
    ),
    schema_field(
        "adapter.object_s3.allow_http",
        "bool",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.object_s3.path_style",
        "bool",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.object_s3.access_key_secret",
        "name",
        Some(1),
        Some(128),
        true,
        None,
    ),
    schema_field(
        "adapter.object_s3.secret_key_secret",
        "name",
        Some(1),
        Some(128),
        true,
        None,
    ),
    schema_field(
        "adapter.object_s3.limits",
        "object",
        None,
        None,
        false,
        Some("object-limits"),
    ),
];
const ADAPTER_QUEUE_DATA_FIELDS: &[DeploymentSchemaField] = &[
    schema_field(
        "adapter.durable_queue_data.kind",
        "literal:durable_queue_data",
        None,
        None,
        false,
        None,
    ),
    schema_field(
        "adapter.durable_queue_data.root",
        "relative-path",
        Some(1),
        Some(4096),
        false,
        None,
    ),
    schema_field(
        "adapter.durable_queue_data.namespace",
        "deployment-token",
        Some(1),
        Some(128),
        false,
        None,
    ),
    schema_field(
        "adapter.durable_queue_data.data_limits",
        "object",
        None,
        None,
        false,
        Some("data-limits"),
    ),
    schema_field(
        "adapter.durable_queue_data.limits",
        "object",
        None,
        None,
        false,
        Some("queue-limits"),
    ),
];

pub(crate) const DEPLOYMENT_ADAPTER_SCHEMAS: &[DeploymentAdapterSchema] = &[
    DeploymentAdapterSchema {
        kind: "configuration",
        fields: ADAPTER_CONFIGURATION_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "wall_clock",
        fields: ADAPTER_WALL_CLOCK_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "secure_random",
        fields: ADAPTER_SECURE_RANDOM_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "identifier",
        fields: ADAPTER_IDENTIFIER_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "password_hash",
        fields: ADAPTER_PASSWORD_HASH_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "secret_verifier",
        fields: ADAPTER_SECRET_VERIFIER_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "byte_stream",
        fields: ADAPTER_BYTE_STREAM_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "http_client",
        fields: ADAPTER_HTTP_CLIENT_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "data",
        fields: ADAPTER_DATA_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "object_memory",
        fields: ADAPTER_OBJECT_MEMORY_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "object_local",
        fields: ADAPTER_OBJECT_LOCAL_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "object_s3",
        fields: ADAPTER_OBJECT_S3_FIELDS,
    },
    DeploymentAdapterSchema {
        kind: "durable_queue_data",
        fields: ADAPTER_QUEUE_DATA_FIELDS,
    },
];

pub(crate) fn starter_http_deployment() -> Result<DeploymentDescriptor, Diagnostic> {
    let descriptor = DeploymentDescriptor {
        contract_version: DEPLOYMENT_CONTRACT_VERSION,
        artifact: STARTER_HTTP_ARTIFACT_PATH.to_owned(),
        target: STARTER_HTTP_TARGET.to_owned(),
        listen: Some(STARTER_HTTP_LISTENER.to_owned()),
        runtime: ResidentLimits {
            maximum_concurrent_tasks: 16,
            maximum_queued_tasks: 64,
            request_deadline_milliseconds: 30_000,
            shutdown_grace_milliseconds: 30_000,
            cancellation_grace_milliseconds: 5_000,
            ..ResidentLimits::default()
        },
        execution: RunPolicy::default(),
        http: Some(HttpLimits {
            maximum_request_body_bytes: 8 * 1024 * 1024,
            maximum_response_body_bytes: 4 * 1024 * 1024,
            maximum_header_bytes: 32 * 1024,
            maximum_headers: 128,
            ..HttpLimits::default()
        }),
        session: None,
        worker: None,
        streams: StreamLimits {
            maximum_chunk_bytes: 64 * 1024,
            maximum_buffered_chunks: 8,
            maximum_total_bytes: 64 * 1024 * 1024,
            maximum_live_streams: 1_024,
        },
        configuration: BTreeMap::new(),
        secrets: Vec::new(),
        grants: vec![DeploymentGrant {
            requirement: "streams".to_owned(),
            sharing_domain: "http-request-streams".to_owned(),
            authority_revision: DeploymentAuthorityRevision::generate()?.encode(),
            adapter: AdapterDescriptor::ByteStream,
        }],
    };
    validate_descriptor(&descriptor)?;
    Ok(descriptor)
}

pub(crate) fn starter_nostr_relay_deployment(
    endpoint: &str,
    address_policy: HttpClientAddressPolicy,
) -> Result<DeploymentDescriptor, Diagnostic> {
    let mut descriptor = starter_http_deployment()?;
    descriptor.grants.push(DeploymentGrant {
        requirement: "relay".to_owned(),
        sharing_domain: "nostr-relay-info-endpoint".to_owned(),
        authority_revision: DeploymentAuthorityRevision::generate()?.encode(),
        adapter: AdapterDescriptor::HttpClient {
            endpoint: endpoint.to_owned(),
            address_policy,
            trust: HttpClientTrust::WebpkiRoots,
            limits: HttpClientLimits::default(),
        },
    });
    validate_descriptor(&descriptor)?;
    Ok(descriptor)
}

pub(crate) fn encode_deployment(descriptor: &DeploymentDescriptor) -> Result<Vec<u8>, Diagnostic> {
    validate_descriptor(descriptor)?;
    let mut bytes = serde_json::to_vec_pretty(descriptor).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "deployment_encode",
            format!("deployment descriptor could not be encoded: {error}"),
        )
    })?;
    bytes.push(b'\n');
    if bytes.len() > MAXIMUM_DEPLOYMENT_BYTES {
        return Err(deployment_error(
            "deployment_too_large",
            format!(
                "deployment descriptor has {} bytes; the limit is {MAXIMUM_DEPLOYMENT_BYTES}",
                bytes.len()
            ),
        ));
    }
    let _ = decode_deployment(&bytes)?;
    Ok(bytes)
}

impl AdapterDescriptor {
    fn kind(&self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::WallClock => "wall-clock",
            Self::SecureRandom => "secure-random",
            Self::Identifier => "identifier",
            Self::PasswordHash { .. } => "password-hash",
            Self::SecretVerifier { .. } => "secret-verifier",
            Self::ByteStream => "byte-stream",
            Self::HttpClient { .. } => "http-client",
            Self::Data { .. } => "data",
            Self::ObjectMemory { .. } => "object-memory",
            Self::ObjectLocal { .. } => "object-local",
            Self::ObjectS3 { .. } => "object-s3",
            Self::DurableQueueData { .. } => "durable-queue-data",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentObservation {
    #[serde(skip_serializing)]
    pub contract_version: u16,
    pub artifact_digest: String,
    pub target: String,
    pub runner: String,
    pub listen: Option<String>,
    pub configuration: ConfigurationObservation,
    pub secret_names: Vec<String>,
    pub grants: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct PreparedDeployment {
    descriptor: DeploymentDescriptor,
    program: Arc<NormalizedProgram>,
    deployment: NormalizedPreparedDeployment,
    observation: DeploymentObservation,
}

impl std::fmt::Debug for PreparedDeployment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedDeployment")
            .field("observation", &self.observation)
            .finish()
    }
}

impl PreparedDeployment {
    pub fn load(path: &Path, runtime: Handle) -> Result<Self, Diagnostic> {
        let descriptor_bytes = read_bounded(
            path,
            MAXIMUM_DEPLOYMENT_BYTES as u64,
            "deployment descriptor",
        )?;
        let descriptor = decode_deployment(&descriptor_bytes)?;
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let artifact_path = resolve_relative(directory, &descriptor.artifact, "artifact")?;
        let artifact_bytes = read_bounded(
            &artifact_path,
            MAXIMUM_ARTIFACT_BUNDLE_BYTES,
            "component artifact",
        )?;
        let artifact = load_artifact(&artifact_bytes)?;
        let artifact_digest = artifact.bundle_digest.to_string();
        let program = Arc::new(NormalizedProgram::prepare(artifact)?);

        // Resolve target, runner, exact requirements, adapter kinds, and grant closure before
        // reading any named secret from the process environment.
        validate_program_descriptor(&descriptor, &program)?;
        let secrets = SecretCatalog::from_environment(&descriptor.secrets)?;
        Self::prepare(
            descriptor,
            program,
            artifact_digest,
            directory,
            runtime,
            secrets,
        )
    }

    fn prepare(
        descriptor: DeploymentDescriptor,
        program: Arc<NormalizedProgram>,
        artifact_digest: String,
        deployment_directory: &Path,
        runtime: Handle,
        secrets: SecretCatalog,
    ) -> Result<Self, Diagnostic> {
        let target_name = Name::new(descriptor.target.clone())?;
        let target = program.root_target(&target_name).cloned().ok_or_else(|| {
            deployment_error(
                "deployment_target_missing",
                "deployment names no exact root-package artifact target",
            )
        })?;
        let component = program
            .components
            .get(target.component.0 as usize)
            .ok_or_else(|| {
                deployment_error(
                    "deployment_component_missing",
                    "selected target component escaped the exact artifact table",
                )
            })?;
        let configuration = ConfigurationStore::observe_values(&descriptor.configuration)?;
        let mut supplied = descriptor
            .grants
            .iter()
            .map(|grant| (grant.requirement.as_str(), grant))
            .collect::<BTreeMap<_, _>>();
        let mut grants = Vec::with_capacity(component.requirements.len());
        let mut observed_grants = BTreeMap::new();
        for requirement_index in component.requirements.iter().copied() {
            let requirement = program
                .requirements
                .get(requirement_index.0 as usize)
                .ok_or_else(|| {
                    deployment_error(
                        "deployment_requirement_missing",
                        "component requirement escaped the exact artifact table",
                    )
                })?;
            let alias = requirement.name.as_str();
            let declared = supplied.remove(alias).ok_or_else(|| {
                deployment_error(
                    "deployment_grant_missing",
                    format!("component requirement '{alias}' has no deployment grant"),
                )
            })?;
            grants.push(NormalizedDeploymentGrant {
                requirement: requirement.reference,
                sharing_domain: NormalizedSharingDomain::new(declared.sharing_domain.clone())?,
                authority_revision: NormalizedGrantAuthorityRevision::of(
                    declared.authority_revision.as_bytes(),
                ),
                limits: requirement
                    .limits
                    .iter()
                    .map(|limit| {
                        (
                            limit.name.clone(),
                            NormalizedGrantLimit {
                                maximum: limit.maximum,
                                unit: limit.unit,
                            },
                        )
                    })
                    .collect(),
                adapter: normalized_adapter(&declared.adapter, &descriptor.configuration),
            });
            observed_grants.insert(alias.to_owned(), declared.adapter.kind().to_owned());
        }
        if let Some((alias, _)) = supplied.into_iter().next() {
            return Err(deployment_error(
                "deployment_grant_foreign",
                format!("deployment grants undeclared component requirement '{alias}'"),
            ));
        }
        let deployment = NormalizedPreparedDeployment::prepare_with_host(
            &program,
            target_name,
            grants,
            NormalizedDeploymentResourcePolicy {
                streams: descriptor.streams.clone(),
            },
            &secrets,
            deployment_directory,
            runtime,
        )?;
        let observation = DeploymentObservation {
            contract_version: DEPLOYMENT_CONTRACT_VERSION,
            artifact_digest,
            target: descriptor.target.clone(),
            runner: format!("{:?}", target.runner).to_ascii_lowercase(),
            listen: descriptor.listen.clone(),
            configuration,
            secret_names: secrets.names(),
            grants: observed_grants,
        };
        Ok(Self {
            descriptor,
            program,
            deployment,
            observation,
        })
    }

    pub fn observe_redacted(&self) -> &DeploymentObservation {
        &self.observation
    }

    pub fn listen(&self) -> Option<&str> {
        self.descriptor.listen.as_deref()
    }

    fn resident(&self) -> Result<NormalizedResidentDeployment, Diagnostic> {
        NormalizedResidentDeployment::prepare(
            Arc::clone(&self.program),
            self.deployment.clone(),
            self.descriptor.runtime.clone(),
            normalized_run_policy(self.descriptor.execution),
        )
    }

    pub fn http_application(&self) -> Result<PreparedHttpApplication, Diagnostic> {
        let limits = self.descriptor.http.clone().ok_or_else(|| {
            deployment_error(
                "deployment_http_missing",
                "HTTP target requires an HTTP limits descriptor",
            )
        })?;
        NormalizedHttpApplication::new(self.resident()?, limits).map(PreparedHttpApplication)
    }

    pub fn worker_application(&self) -> Result<PreparedWorkerApplication, Diagnostic> {
        let limits = self.descriptor.worker.clone().ok_or_else(|| {
            deployment_error(
                "deployment_worker_missing",
                "worker target requires a worker limits descriptor",
            )
        })?;
        NormalizedWorkerApplication::new(self.resident()?, limits).map(PreparedWorkerApplication)
    }

    pub fn session_application(&self) -> Result<PreparedSessionApplication, Diagnostic> {
        let limits = self.descriptor.session.clone().ok_or_else(|| {
            deployment_error(
                "deployment_session_missing",
                "interactive target requires a structured-session limits descriptor",
            )
        })?;
        NormalizedSessionApplication::new(self.resident()?, limits).map(PreparedSessionApplication)
    }
}

#[derive(Clone)]
pub struct PreparedHttpApplication(NormalizedHttpApplication);

impl PreparedHttpApplication {
    pub async fn dispatch(
        &self,
        request: HttpRequest,
    ) -> Result<(HttpResponse, HttpDispatchObservation), super::execution::ExecutionError> {
        self.0.dispatch(request).await
    }

    pub async fn serve(
        self,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<HttpServerReceipt, Diagnostic> {
        self.0.serve(listener, shutdown).await
    }

    pub fn observe_resident(&self) -> ResidentObservation {
        self.0.resident().observe()
    }

    pub async fn shutdown(&self) -> ShutdownReceipt {
        self.0.resident().shutdown().await
    }
}

#[derive(Clone)]
pub struct PreparedWorkerApplication(NormalizedWorkerApplication);

impl PreparedWorkerApplication {
    pub async fn run(
        self,
        shutdown: impl Future<Output = ()> + Send,
    ) -> Result<WorkerReceipt, Diagnostic> {
        self.0.run(shutdown).await
    }

    pub fn observe_resident(&self) -> ResidentObservation {
        self.0.resident().observe()
    }

    pub async fn shutdown(&self) -> ShutdownReceipt {
        self.0.resident().shutdown().await
    }
}

#[derive(Clone)]
pub struct PreparedSessionApplication(NormalizedSessionApplication);

impl PreparedSessionApplication {
    pub async fn serve(
        self,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<SessionServerReceipt, Diagnostic> {
        self.0.serve(listener, shutdown).await
    }

    pub fn observe_sessions(&self) -> SessionObservation {
        self.0.observe()
    }

    pub fn observe_resident(&self) -> ResidentObservation {
        self.0.resident().observe()
    }

    pub async fn shutdown(&self) -> ShutdownReceipt {
        self.0.shutdown().await
    }
}

pub fn decode_deployment(bytes: &[u8]) -> Result<DeploymentDescriptor, Diagnostic> {
    if bytes.len() > MAXIMUM_DEPLOYMENT_BYTES {
        return Err(deployment_error(
            "deployment_too_large",
            format!(
                "deployment descriptor has {} bytes; the limit is {MAXIMUM_DEPLOYMENT_BYTES}",
                bytes.len()
            ),
        ));
    }
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let descriptor = DeploymentDescriptor::deserialize(&mut decoder).map_err(|error| {
        deployment_error(
            "deployment_json",
            format!("deployment descriptor is not strict JSON: {error}"),
        )
    })?;
    decoder.end().map_err(|error| {
        deployment_error(
            "deployment_trailing_json",
            format!("deployment descriptor has trailing input: {error}"),
        )
    })?;
    validate_raw_adapter_fields(bytes)?;
    validate_descriptor(&descriptor)?;
    Ok(descriptor)
}

fn validate_raw_adapter_fields(bytes: &[u8]) -> Result<(), Diagnostic> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        deployment_error(
            "deployment_json",
            format!("deployment descriptor is not strict JSON: {error}"),
        )
    })?;
    let root = value.as_object().ok_or_else(|| {
        deployment_error(
            "deployment_json",
            "deployment descriptor must be one strict JSON object",
        )
    })?;
    let expected_root = DEPLOYMENT_SCHEMA_FIELDS
        .iter()
        .filter_map(|field| field.path.strip_prefix("deployment."))
        .filter(|field| !field.contains('.'))
        .collect::<BTreeSet<_>>();
    if root.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected_root {
        return Err(deployment_error(
            "deployment_json",
            "deployment descriptor has a missing, duplicate, or unknown top-level field",
        ));
    }
    let grants = value
        .get("grants")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            deployment_error(
                "deployment_json",
                "deployment grants must be a strict JSON array",
            )
        })?;
    for grant in grants {
        let adapter = grant
            .get("adapter")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| {
                deployment_error(
                    "deployment_json",
                    "deployment grant adapter must be a strict tagged object",
                )
            })?;
        let kind = adapter
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                deployment_error(
                    "deployment_json",
                    "deployment grant adapter requires one string kind",
                )
            })?;
        let schema = DEPLOYMENT_ADAPTER_SCHEMAS
            .iter()
            .find(|schema| schema.kind == kind)
            .ok_or_else(|| {
                deployment_error(
                    "deployment_json",
                    "deployment grant adapter names an unknown kind",
                )
            })?;
        let expected = schema
            .fields
            .iter()
            .filter_map(|field| field.path.rsplit('.').next())
            .collect::<BTreeSet<_>>();
        if adapter.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
            return Err(deployment_error(
                "deployment_json",
                "deployment grant adapter has a missing, duplicate, or unknown field",
            ));
        }
    }
    Ok(())
}

fn validate_descriptor(descriptor: &DeploymentDescriptor) -> Result<(), Diagnostic> {
    if descriptor.contract_version != DEPLOYMENT_CONTRACT_VERSION {
        return Err(deployment_error(
            "deployment_contract",
            format!(
                "deployment contract {} is not current contract {DEPLOYMENT_CONTRACT_VERSION}",
                descriptor.contract_version
            ),
        ));
    }
    validate_relative(&descriptor.artifact, "artifact")?;
    validate_name(&descriptor.target, "target")?;
    if descriptor.grants.len() > MAXIMUM_DEPLOYMENT_GRANTS {
        return Err(deployment_error(
            "deployment_grant_limit",
            format!("deployment has more than {MAXIMUM_DEPLOYMENT_GRANTS} grants"),
        ));
    }
    if let Some(listen) = &descriptor.listen
        && (listen.is_empty() || listen.len() > 512 || listen.contains('\0'))
    {
        return Err(deployment_error(
            "deployment_listener",
            "listener descriptor is empty, excessive, or contains NUL",
        ));
    }
    descriptor.runtime.validate()?;
    descriptor.streams.validate()?;
    if descriptor.execution.instruction_fuel == 0
        || descriptor.execution.maximum_call_depth == 0
        || descriptor.execution.maximum_value_stack == 0
    {
        return Err(deployment_error(
            "deployment_execution_limit",
            "execution fuel, call depth, and value stack limits must be positive",
        ));
    }
    if let Some(http) = &descriptor.http {
        http.validate()?;
    }
    if let Some(session) = &descriptor.session {
        session.validate()?;
    }
    if let Some(worker) = &descriptor.worker {
        worker.validate(descriptor.runtime.maximum_concurrent_tasks)?;
    }
    ConfigurationStore::observe_values(&descriptor.configuration)?;
    if descriptor.secrets.len() > MAXIMUM_DEPLOYMENT_GRANTS {
        return Err(deployment_error(
            "deployment_secret_limit",
            format!("deployment has more than {MAXIMUM_DEPLOYMENT_GRANTS} secret bindings"),
        ));
    }
    SecretCatalog::validate_bindings(&descriptor.secrets)?;
    let mut requirements = BTreeSet::new();
    for grant in &descriptor.grants {
        validate_name(&grant.requirement, "requirement")?;
        validate_name(&grant.sharing_domain, "sharing domain")?;
        validate_digest(&grant.authority_revision, "authority revision")?;
        if !requirements.insert(grant.requirement.as_str()) {
            return Err(deployment_error(
                "deployment_grant_duplicate",
                format!(
                    "deployment requirement '{}' is granted twice",
                    grant.requirement
                ),
            ));
        }
        validate_adapter_descriptor(&grant.adapter)?;
    }
    Ok(())
}

fn validate_adapter_descriptor(adapter: &AdapterDescriptor) -> Result<(), Diagnostic> {
    match adapter {
        AdapterDescriptor::PasswordHash { policy } => policy.validate()?,
        AdapterDescriptor::SecretVerifier {
            secret,
            maximum_candidate_bytes,
        } => {
            validate_name(secret, "secret name")?;
            if *maximum_candidate_bytes == 0 || *maximum_candidate_bytes > MAXIMUM_SECRET_BYTES {
                return Err(deployment_error(
                    "deployment_secret_candidate_limit",
                    format!(
                        "secret candidate limit must be 1 through {MAXIMUM_SECRET_BYTES} bytes"
                    ),
                ));
            }
        }
        AdapterDescriptor::HttpClient {
            endpoint,
            address_policy,
            trust,
            limits,
        } => {
            validate_http_client_descriptor(endpoint, *address_policy, limits)?;
            if let HttpClientTrust::NamedPemRoot { secret } = trust {
                validate_name(secret, "HTTP client root secret name")?;
            }
        }
        AdapterDescriptor::Data {
            root,
            namespace,
            limits,
        } => {
            validate_relative(root, "data root")?;
            validate_deployment_token(namespace, "data namespace", 128)?;
            limits.validate()?;
        }
        AdapterDescriptor::ObjectMemory { prefix, limits } => {
            validate_object_prefix(prefix)?;
            limits.validate()?;
        }
        AdapterDescriptor::ObjectLocal {
            root,
            prefix,
            limits,
        } => {
            validate_relative(root, "object root")?;
            validate_object_prefix(prefix)?;
            limits.validate()?;
        }
        AdapterDescriptor::ObjectS3 {
            endpoint,
            region,
            bucket,
            prefix,
            access_key_secret,
            secret_key_secret,
            limits,
            ..
        } => {
            if endpoint.is_empty() || endpoint.len() > 4096 {
                return Err(deployment_error(
                    "deployment_object_endpoint",
                    "object endpoint must contain 1 through 4096 bytes",
                ));
            }
            validate_deployment_token(region, "object region", 255)?;
            validate_deployment_token(bucket, "object bucket", 255)?;
            validate_object_prefix(prefix)?;
            validate_name(access_key_secret, "access-key secret name")?;
            validate_name(secret_key_secret, "secret-key secret name")?;
            limits.validate()?;
        }
        AdapterDescriptor::DurableQueueData {
            root,
            namespace,
            data_limits,
            limits,
        } => {
            validate_relative(root, "queue data root")?;
            validate_deployment_token(namespace, "queue namespace", 128)?;
            data_limits.validate()?;
            limits.validate()?;
        }
        AdapterDescriptor::Configuration
        | AdapterDescriptor::WallClock
        | AdapterDescriptor::SecureRandom
        | AdapterDescriptor::Identifier
        | AdapterDescriptor::ByteStream => {}
    }
    Ok(())
}

fn validate_object_prefix(prefix: &str) -> Result<(), Diagnostic> {
    if prefix.is_empty() {
        return Ok(());
    }
    if prefix.len() > MAXIMUM_OBJECT_KEY_BYTES
        || prefix.starts_with('/')
        || prefix.ends_with('/')
        || prefix.contains('\0')
        || prefix
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(deployment_error(
            "deployment_object_prefix",
            "object prefix is excessive or noncanonical",
        ));
    }
    Ok(())
}

fn validate_deployment_token(value: &str, label: &str, maximum: usize) -> Result<(), Diagnostic> {
    if value.is_empty()
        || value.len() > maximum
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
    {
        return Err(deployment_error(
            "deployment_token",
            format!("{label} is not a canonical bounded deployment token"),
        ));
    }
    Ok(())
}

fn validate_program_descriptor(
    descriptor: &DeploymentDescriptor,
    program: &NormalizedProgram,
) -> Result<(), Diagnostic> {
    let target_name = Name::new(descriptor.target.clone())?;
    let target = program.root_target(&target_name).ok_or_else(|| {
        deployment_error(
            "deployment_target_missing",
            "deployment names no exact root-package artifact target",
        )
    })?;
    validate_runner_descriptor(descriptor, target.runner)?;
    let component = program
        .components
        .get(target.component.0 as usize)
        .ok_or_else(|| {
            deployment_error(
                "deployment_component_missing",
                "selected target component escaped the exact artifact table",
            )
        })?;
    let supplied = descriptor
        .grants
        .iter()
        .map(|grant| (grant.requirement.as_str(), grant))
        .collect::<BTreeMap<_, _>>();
    for requirement_index in component.requirements.iter().copied() {
        let requirement = program
            .requirements
            .get(requirement_index.0 as usize)
            .ok_or_else(|| {
                deployment_error(
                    "deployment_requirement_missing",
                    "component requirement escaped the exact artifact table",
                )
            })?;
        let grant = supplied.get(requirement.name.as_str()).ok_or_else(|| {
            deployment_error(
                "deployment_grant_missing",
                format!(
                    "component requirement '{}' has no deployment grant",
                    requirement.name
                ),
            )
        })?;
        validate_exact_adapter_interface(requirement.interface, &grant.adapter)?;
    }
    if supplied.len() != component.requirements.len() {
        let required = component
            .requirements
            .iter()
            .filter_map(|index| program.requirements.get(index.0 as usize))
            .map(|requirement| requirement.name.as_str())
            .collect::<BTreeSet<_>>();
        let foreign = supplied
            .keys()
            .find(|alias| !required.contains(**alias))
            .copied()
            .unwrap_or("<unknown>");
        return Err(deployment_error(
            "deployment_grant_foreign",
            format!("deployment grants undeclared component requirement '{foreign}'"),
        ));
    }
    Ok(())
}

fn validate_exact_adapter_interface(
    interface: super::kernel::DeclarationReference,
    adapter: &AdapterDescriptor,
) -> Result<(), Diagnostic> {
    const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
    let declaration = match adapter {
        AdapterDescriptor::Configuration => "decl_def8eec5eed34e86eda0df7ee7bb4883",
        AdapterDescriptor::WallClock => "decl_8d99ab2f1d59391e1e21c17cc8757731",
        AdapterDescriptor::SecureRandom => "decl_2ad39598d2945149fff8b841fe8b253e",
        AdapterDescriptor::Identifier => "decl_92bb73b52bc3654abcbde47513873f42",
        AdapterDescriptor::PasswordHash { .. } => "decl_375bc0a9f5214e8a27ede17a14e79f67",
        AdapterDescriptor::SecretVerifier { .. } => "decl_172ae7f44000b32243d75a92e6733e50",
        AdapterDescriptor::ByteStream => "decl_e29e0ac407696662f355e9056172ac2b",
        AdapterDescriptor::HttpClient { .. } => "decl_f1084ba5dca02ba338140747d0ea9d46",
        AdapterDescriptor::Data { .. } => "decl_640e96fa57dee1c09557eb4bc7b53398",
        AdapterDescriptor::ObjectMemory { .. }
        | AdapterDescriptor::ObjectLocal { .. }
        | AdapterDescriptor::ObjectS3 { .. } => "decl_ac421d578f44958595e92fa9f5fb1d43",
        AdapterDescriptor::DurableQueueData { .. } => "decl_20a0ef729beda0abf0e743cd7e1126de",
    };
    if interface.package.to_string() != STANDARD_PACKAGE
        || interface.declaration.to_string() != declaration
    {
        return Err(deployment_error(
            "deployment_adapter_interface",
            format!(
                "{} adapter requires its exact maintained standard interface",
                adapter.kind()
            ),
        ));
    }
    Ok(())
}

fn validate_runner_descriptor(
    descriptor: &DeploymentDescriptor,
    runner: RunnerKind,
) -> Result<(), Diagnostic> {
    match runner {
        RunnerKind::Http => {
            if descriptor.listen.is_none() || descriptor.http.is_none() {
                return Err(deployment_error(
                    "deployment_http_incomplete",
                    "HTTP target requires listen and http descriptors",
                ));
            }
            if descriptor.worker.is_some() || descriptor.session.is_some() {
                return Err(deployment_error(
                    "deployment_runner_foreign",
                    "HTTP target may not declare worker or interactive topology",
                ));
            }
        }
        RunnerKind::Interactive => {
            if descriptor.listen.is_none() || descriptor.session.is_none() {
                return Err(deployment_error(
                    "deployment_session_incomplete",
                    "interactive target requires listen and session descriptors",
                ));
            }
            if descriptor.http.is_some() || descriptor.worker.is_some() {
                return Err(deployment_error(
                    "deployment_runner_foreign",
                    "interactive target may not declare HTTP or worker topology",
                ));
            }
        }
        RunnerKind::Worker => {
            if descriptor.worker.is_none() {
                return Err(deployment_error(
                    "deployment_worker_incomplete",
                    "worker target requires a worker descriptor",
                ));
            }
            if descriptor.listen.is_some()
                || descriptor.http.is_some()
                || descriptor.session.is_some()
            {
                return Err(deployment_error(
                    "deployment_runner_foreign",
                    "worker target may not declare listener, HTTP, or interactive topology",
                ));
            }
        }
        RunnerKind::Command | RunnerKind::Batch | RunnerKind::Test => {
            if descriptor.listen.is_some()
                || descriptor.http.is_some()
                || descriptor.session.is_some()
                || descriptor.worker.is_some()
            {
                return Err(deployment_error(
                    "deployment_runner_foreign",
                    "nonresident target may not declare HTTP, interactive, or worker topology",
                ));
            }
        }
    }
    Ok(())
}

fn normalized_adapter(
    adapter: &AdapterDescriptor,
    configuration: &BTreeMap<String, ConfigurationValue>,
) -> NormalizedAdapterDescriptor {
    match adapter {
        AdapterDescriptor::Configuration => NormalizedAdapterDescriptor::Configuration {
            values: configuration.clone(),
        },
        AdapterDescriptor::WallClock => NormalizedAdapterDescriptor::WallClock,
        AdapterDescriptor::SecureRandom => NormalizedAdapterDescriptor::SecureRandom,
        AdapterDescriptor::Identifier => NormalizedAdapterDescriptor::Identifier,
        AdapterDescriptor::PasswordHash { policy } => NormalizedAdapterDescriptor::PasswordHash {
            policy: policy.clone(),
        },
        AdapterDescriptor::SecretVerifier {
            secret,
            maximum_candidate_bytes,
        } => NormalizedAdapterDescriptor::SecretVerifier {
            secret: secret.clone(),
            maximum_candidate_bytes: *maximum_candidate_bytes,
        },
        AdapterDescriptor::ByteStream => NormalizedAdapterDescriptor::ByteStream,
        AdapterDescriptor::HttpClient {
            endpoint,
            address_policy,
            trust,
            limits,
        } => NormalizedAdapterDescriptor::HttpClient {
            endpoint: endpoint.clone(),
            address_policy: *address_policy,
            trust: trust.clone(),
            limits: limits.clone(),
        },
        AdapterDescriptor::Data {
            root,
            namespace,
            limits,
        } => NormalizedAdapterDescriptor::Data {
            root: root.clone(),
            namespace: namespace.clone(),
            limits: limits.clone(),
        },
        AdapterDescriptor::ObjectMemory { prefix, limits } => {
            NormalizedAdapterDescriptor::ObjectMemory {
                prefix: prefix.clone(),
                limits: limits.clone(),
            }
        }
        AdapterDescriptor::ObjectLocal {
            root,
            prefix,
            limits,
        } => NormalizedAdapterDescriptor::ObjectLocal {
            root: root.clone(),
            prefix: prefix.clone(),
            limits: limits.clone(),
        },
        AdapterDescriptor::ObjectS3 {
            endpoint,
            region,
            bucket,
            prefix,
            allow_http,
            path_style,
            access_key_secret,
            secret_key_secret,
            limits,
        } => NormalizedAdapterDescriptor::ObjectS3 {
            endpoint: endpoint.clone(),
            region: region.clone(),
            bucket: bucket.clone(),
            prefix: prefix.clone(),
            allow_http: *allow_http,
            path_style: *path_style,
            access_key_secret: access_key_secret.clone(),
            secret_key_secret: secret_key_secret.clone(),
            limits: limits.clone(),
        },
        AdapterDescriptor::DurableQueueData {
            root,
            namespace,
            data_limits,
            limits,
        } => NormalizedAdapterDescriptor::DurableQueueData {
            root: root.clone(),
            namespace: namespace.clone(),
            data_limits: data_limits.clone(),
            limits: limits.clone(),
        },
    }
}

fn normalized_run_policy(policy: RunPolicy) -> NormalizedRunPolicy {
    NormalizedRunPolicy {
        instruction_steps: policy.instruction_fuel,
        maximum_call_depth: policy.maximum_call_depth,
        maximum_value_stack: policy.maximum_value_stack,
        ..NormalizedRunPolicy::default()
    }
}

fn resolve_relative(root: &Path, value: &str, label: &str) -> Result<PathBuf, Diagnostic> {
    validate_relative(value, label)?;
    let mut resolved = root.to_path_buf();
    let mut components = Path::new(value).components().peekable();
    while let Some(Component::Normal(component)) = components.next() {
        resolved.push(component);
        match fs::symlink_metadata(&resolved) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(deployment_error(
                    "deployment_input_kind",
                    format!("{label} path contains a symbolic-link component"),
                ));
            }
            Ok(metadata) if components.peek().is_some() && !metadata.is_dir() => {
                return Err(deployment_error(
                    "deployment_input_kind",
                    format!("{label} path contains a non-directory parent component"),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(root.join(value));
            }
            Err(error) => return Err(deployment_io("deployment_read", &resolved, error)),
        }
    }
    Ok(resolved)
}

fn validate_relative(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.is_empty() || value.len() > 4096 || value.contains('\0') || value.contains('\\') {
        return Err(deployment_error(
            "deployment_path",
            format!("{label} path is empty, excessive, or noncanonical"),
        ));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(deployment_error(
            "deployment_path",
            format!("{label} path is not a canonical relative path"),
        ));
    }
    Ok(())
}

fn validate_name(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(deployment_error(
            "deployment_name",
            format!("{label} is not a canonical bounded name"),
        ));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(deployment_error(
            "deployment_digest",
            format!("{label} must be 64 lowercase hexadecimal characters"),
        ));
    }
    Ok(())
}

fn read_bounded(path: &Path, maximum: u64, label: &str) -> Result<Vec<u8>, Diagnostic> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| deployment_io("deployment_read", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(deployment_error(
            "deployment_input_kind",
            format!(
                "{label} '{}' is not a regular non-symlink file",
                path.display()
            ),
        ));
    }
    if metadata.len() > maximum {
        return Err(deployment_error(
            "deployment_input_limit",
            format!("{label} '{}' exceeds {maximum} bytes", path.display()),
        ));
    }
    let bytes = fs::read(path).map_err(|error| deployment_io("deployment_read", path, error))?;
    if u64::try_from(bytes.len()).map_or(true, |length| length > maximum) {
        return Err(deployment_error(
            "deployment_input_limit",
            format!("{label} '{}' exceeds {maximum} bytes", path.display()),
        ));
    }
    Ok(bytes)
}

fn deployment_io(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

fn deployment_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn minimal() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "artifact": "application.lkja",
            "target": "serve",
            "listen": "127.0.0.1:0",
            "runtime": ResidentLimits::default(),
            "execution": RunPolicy::default(),
            "http": HttpLimits::default(),
            "session": null,
            "worker": null,
            "streams": StreamLimits::default(),
            "configuration": {},
            "secrets": [],
            "grants": []
        }))
        .expect("deployment JSON")
    }

    fn adapter_samples() -> Vec<AdapterDescriptor> {
        vec![
            AdapterDescriptor::Configuration,
            AdapterDescriptor::WallClock,
            AdapterDescriptor::SecureRandom,
            AdapterDescriptor::Identifier,
            AdapterDescriptor::PasswordHash {
                policy: PasswordHashPolicy::default(),
            },
            AdapterDescriptor::SecretVerifier {
                secret: "candidate-secret".to_owned(),
                maximum_candidate_bytes: 1024,
            },
            AdapterDescriptor::ByteStream,
            AdapterDescriptor::HttpClient {
                endpoint: "https://relay.example/".to_owned(),
                address_policy: HttpClientAddressPolicy::PublicOnly,
                trust: HttpClientTrust::WebpkiRoots,
                limits: HttpClientLimits::default(),
            },
            AdapterDescriptor::Data {
                root: "state/data".to_owned(),
                namespace: "test-data".to_owned(),
                limits: DataLimits::default(),
            },
            AdapterDescriptor::ObjectMemory {
                prefix: "bbs".to_owned(),
                limits: ObjectLimits::default(),
            },
            AdapterDescriptor::ObjectLocal {
                root: "objects".to_owned(),
                prefix: "bbs".to_owned(),
                limits: ObjectLimits::default(),
            },
            AdapterDescriptor::ObjectS3 {
                endpoint: "http://127.0.0.1:9000".to_owned(),
                region: "test-region".to_owned(),
                bucket: "test-bucket".to_owned(),
                prefix: "bbs".to_owned(),
                allow_http: true,
                path_style: true,
                access_key_secret: "object-access".to_owned(),
                secret_key_secret: "object-secret".to_owned(),
                limits: ObjectLimits::default(),
            },
            AdapterDescriptor::DurableQueueData {
                root: "state/data".to_owned(),
                namespace: "test:queue".to_owned(),
                data_limits: DataLimits::default(),
                limits: QueueLimits::default(),
            },
        ]
    }

    fn deployment_with_adapter(adapter: &AdapterDescriptor) -> serde_json::Value {
        let mut value: serde_json::Value =
            serde_json::from_slice(&minimal()).expect("minimal deployment");
        value["grants"] = serde_json::json!([{
            "requirement": "adapter",
            "sharing_domain": "isolated",
            "authority_revision": "11".repeat(32),
            "adapter": serde_json::to_value(adapter).expect("adapter JSON")
        }]);
        value
    }

    #[test]
    fn strict_current_descriptor_and_relative_paths_are_enforced() {
        assert!(decode_deployment(&minimal()).is_ok());
        let mut value: serde_json::Value =
            serde_json::from_slice(&minimal()).expect("deployment value");
        for path in ["top", "runtime", "http"] {
            let mut predecessor = value.clone();
            match path {
                "top" => predecessor["contract_version"] = serde_json::json!(1),
                "runtime" => predecessor["runtime"]["contract_version"] = serde_json::json!(1),
                "http" => predecessor["http"]["contract_version"] = serde_json::json!(1),
                _ => unreachable!(),
            }
            let error = decode_deployment(
                &serde_json::to_vec(&predecessor).expect("predecessor deployment JSON"),
            )
            .expect_err("removed predecessor field must reject");
            assert_eq!(error.code, "deployment_json", "{path}");
        }
        let mut worker = value.clone();
        worker["http"] = serde_json::Value::Null;
        worker["worker"] = serde_json::json!({
            "maximum_workers": 1,
            "idle_wait_milliseconds": 100,
            "contract_version": 1
        });
        assert_eq!(
            decode_deployment(&serde_json::to_vec(&worker).expect("worker predecessor JSON"))
                .expect_err("removed worker predecessor field must reject")
                .code,
            "deployment_json"
        );
        value["artifact"] = serde_json::json!("../foreign.lkja");
        let error = decode_deployment(&serde_json::to_vec(&value).expect("path JSON"))
            .expect_err("traversal must reject");
        assert_eq!(error.code, "deployment_path");
        value["artifact"] = serde_json::json!("foreign\\artifact.lkja");
        assert_eq!(
            decode_deployment(&serde_json::to_vec(&value).expect("backslash path JSON"))
                .expect_err("backslash must reject")
                .code,
            "deployment_path"
        );

        for path in ["", "/foreign.lkja", ".", "nested/../foreign.lkja"] {
            value["artifact"] = serde_json::json!(path);
            assert_eq!(
                decode_deployment(&serde_json::to_vec(&value).expect("path JSON"))
                    .expect_err("noncanonical path must reject")
                    .code,
                "deployment_path",
                "{path}"
            );
        }
    }

    #[test]
    fn starter_http_descriptor_is_strict_loopback_only_and_fresh() {
        let first = starter_http_deployment().expect("first starter deployment");
        let second = starter_http_deployment().expect("second starter deployment");
        assert_eq!(first.artifact, STARTER_HTTP_ARTIFACT_PATH);
        assert_eq!(first.target, STARTER_HTTP_TARGET);
        assert_eq!(first.listen.as_deref(), Some(STARTER_HTTP_LISTENER));
        assert_eq!(first.runtime.maximum_concurrent_tasks, 16);
        assert_eq!(first.runtime.maximum_queued_tasks, 64);
        assert_eq!(first.runtime.request_deadline_milliseconds, 30_000);
        assert_eq!(first.runtime.shutdown_grace_milliseconds, 30_000);
        assert_eq!(first.runtime.cancellation_grace_milliseconds, 5_000);
        assert_eq!(first.execution.instruction_fuel, 10_000_000);
        assert_eq!(first.execution.maximum_call_depth, 4_096);
        assert_eq!(first.execution.maximum_value_stack, 1_000_000);
        let http = first.http.as_ref().expect("HTTP limits");
        assert_eq!(http.maximum_request_body_bytes, 8 * 1024 * 1024);
        assert_eq!(http.maximum_response_body_bytes, 4 * 1024 * 1024);
        assert_eq!(http.maximum_header_bytes, 32 * 1024);
        assert_eq!(http.maximum_headers, 128);
        assert_eq!(first.streams.maximum_chunk_bytes, 64 * 1024);
        assert_eq!(first.streams.maximum_buffered_chunks, 8);
        assert_eq!(first.streams.maximum_total_bytes, 64 * 1024 * 1024);
        assert_eq!(first.streams.maximum_live_streams, 1_024);
        assert!(first.worker.is_none());
        assert!(first.configuration.is_empty());
        assert!(first.secrets.is_empty());
        assert_eq!(first.grants.len(), 1);
        let grant = &first.grants[0];
        assert_eq!(grant.requirement, "streams");
        assert_eq!(grant.sharing_domain, "http-request-streams");
        assert_eq!(grant.authority_revision.len(), 64);
        assert_ne!(grant.authority_revision, "0".repeat(64));
        assert!(matches!(grant.adapter, AdapterDescriptor::ByteStream));
        assert_ne!(
            grant.authority_revision, second.grants[0].authority_revision,
            "starter deployment authority must be freshly generated"
        );

        let bytes = encode_deployment(&first).expect("encode starter deployment");
        assert!(!String::from_utf8_lossy(&bytes).contains("contract_version"));
        assert_eq!(bytes.last(), Some(&b'\n'));
        let decoded = decode_deployment(&bytes).expect("strictly decode encoded starter");
        assert_eq!(decoded.artifact, first.artifact);
        assert_eq!(decoded.target, first.target);
        assert_eq!(decoded.listen, first.listen);
        assert_eq!(
            decoded.grants[0].authority_revision,
            grant.authority_revision
        );
    }

    #[test]
    fn executable_adapter_schema_matches_strict_decoder_fields() {
        let samples = adapter_samples();
        assert_eq!(samples.len(), DEPLOYMENT_ADAPTER_SCHEMAS.len());
        for (sample, schema) in samples.iter().zip(DEPLOYMENT_ADAPTER_SCHEMAS) {
            let adapter = serde_json::to_value(sample).expect("adapter JSON");
            assert_eq!(adapter["kind"], schema.kind);
            let observed = adapter
                .as_object()
                .expect("adapter object")
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            let expected = schema
                .fields
                .iter()
                .map(|field| {
                    field
                        .path
                        .rsplit('.')
                        .next()
                        .expect("schema field name")
                        .to_owned()
                })
                .collect::<BTreeSet<_>>();
            assert_eq!(observed, expected, "{} schema fields", schema.kind);
            let value = deployment_with_adapter(sample);
            decode_deployment(&serde_json::to_vec(&value).expect("deployment JSON"))
                .expect("schema sample must decode");
            for field in &expected {
                let mut missing = value.clone();
                missing["grants"][0]["adapter"]
                    .as_object_mut()
                    .expect("adapter object")
                    .remove(field);
                assert_eq!(
                    decode_deployment(&serde_json::to_vec(&missing).expect("missing field JSON"))
                        .expect_err("required adapter field must reject")
                        .code,
                    "deployment_json",
                    "{}.{}",
                    schema.kind,
                    field
                );
            }
            let mut unknown = value;
            unknown["grants"][0]["adapter"]["unknown"] = serde_json::json!(true);
            assert_eq!(
                decode_deployment(&serde_json::to_vec(&unknown).expect("unknown field JSON"))
                    .expect_err("unknown adapter field must reject")
                    .code,
                "deployment_json",
                "{} unknown field",
                schema.kind
            );
        }
    }

    #[test]
    fn predecessor_postgres_adapters_reject_at_strict_decode() {
        for kind in ["postgres", "durable_queue_postgres"] {
            let mut value: serde_json::Value =
                serde_json::from_slice(&minimal()).expect("minimal deployment");
            value["grants"] = serde_json::json!([{
                "requirement": "adapter",
                "sharing_domain": "isolated",
                "authority_revision": "11".repeat(32),
                "adapter": {"kind": kind, "connection_secret": "must-not-be-read"}
            }]);
            assert_eq!(
                decode_deployment(&serde_json::to_vec(&value).expect("predecessor JSON"))
                    .expect_err("predecessor adapter must reject")
                    .code,
                "deployment_json"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn deployment_inputs_reject_symbolic_links_and_nonregular_files() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("deployment input directory");
        let artifact = temporary.path().join("application.lkja");
        fs::write(&artifact, b"artifact").expect("artifact fixture");
        let linked_artifact = temporary.path().join("linked.lkja");
        symlink(&artifact, &linked_artifact).expect("artifact symlink");
        assert_eq!(
            read_bounded(&linked_artifact, 64, "component artifact")
                .expect_err("artifact symlink must reject")
                .code,
            "deployment_input_kind"
        );

        let linked_directory = temporary.path().join("linked-directory");
        symlink(temporary.path(), &linked_directory).expect("directory symlink");
        assert_eq!(
            resolve_relative(
                temporary.path(),
                "linked-directory/application.lkja",
                "artifact"
            )
            .expect_err("parent symlink must reject")
            .code,
            "deployment_input_kind"
        );
        assert_eq!(
            read_bounded(temporary.path(), 64, "component artifact")
                .expect_err("directory input must reject")
                .code,
            "deployment_input_kind"
        );
        assert_eq!(
            read_bounded(&artifact, 1, "component artifact")
                .expect_err("oversized input must reject")
                .code,
            "deployment_input_limit"
        );
    }
}
