mod process;
pub(super) use process::ProcessCode;
pub use process::SessionProcessError;

use serde::{Deserialize, Serialize};

use crate::semantic::schema::{Request, Response, SourceUnitRecord};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionRequest {
    pub schema: String,
    pub contract: String,
    pub request_id: String,
    pub revision: u64,
    pub request: SessionOperation,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum SessionOperation {
    Execute { request: Request },
    Refresh,
    Shutdown,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionResponse {
    pub schema: &'static str,
    pub contract: String,
    pub request_id: String,
    pub revision: u64,
    pub response: SessionResult,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum SessionResult {
    Execute {
        response: Box<Response>,
        session: SessionStateRecord,
    },
    Refresh {
        session: SessionStateRecord,
    },
    Shutdown {
        acknowledged: bool,
    },
    Error {
        error: SessionError,
    },
}

#[derive(Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionStateRecord {
    pub session_identity: String,
    pub compiler_build: String,
    pub semantic_schema: String,
    pub semantic_contract: String,
    pub diagnostic_schema: String,
    pub diagnostic_contract: String,
    pub canonical_root: String,
    pub source_revision: String,
    pub cache_entries: u64,
}

#[derive(Clone)]
pub(super) struct SourceFingerprint {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone)]
pub(super) struct PinnedSession {
    pub state: SessionStateRecord,
    pub fingerprints: Vec<SourceFingerprint>,
}

impl PinnedSession {
    pub fn metadata_bytes(&self) -> Option<u64> {
        let fixed = [
            self.state.session_identity.len(),
            self.state.compiler_build.len(),
            self.state.semantic_schema.len(),
            self.state.semantic_contract.len(),
            self.state.diagnostic_schema.len(),
            self.state.diagnostic_contract.len(),
            self.state.canonical_root.len(),
            self.state.source_revision.len(),
        ]
        .into_iter()
        .try_fold(0_usize, usize::checked_add)?;
        let fixed = u64::try_from(fixed).ok()?;
        self.fingerprints.iter().try_fold(fixed, |total, item| {
            let item_bytes = item
                .path
                .len()
                .checked_add(item.sha256.len())?
                .checked_add(std::mem::size_of_val(&item.bytes))?;
            total.checked_add(u64::try_from(item_bytes).ok()?)
        })
    }
}
impl From<&SourceUnitRecord> for SourceFingerprint {
    fn from(unit: &SourceUnitRecord) -> Self {
        Self {
            path: unit.path.clone(),
            bytes: unit.bytes,
            sha256: unit.sha256.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum SessionErrorCode {
    NotInitialized,
    StaleSessionRevision,
    PinnedRootMismatch,
    ExternalSourceChange,
    ResourceLimit,
    RevisionOverflow,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionError {
    pub code: SessionErrorCode,
    pub message: String,
}

impl SessionError {
    pub fn new(code: SessionErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
