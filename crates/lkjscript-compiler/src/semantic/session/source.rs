use std::path::Path;

use lkjscript_core::BudgetLedger;

use crate::semantic::schema::{
    OperationRequest, Request, ResourceProfile, Response, ResponseResult,
};

use super::schema::{SessionError, SessionErrorCode, SourceFingerprint};

pub(super) struct SourceSnapshot {
    pub response: Response,
    pub revision: String,
    pub fingerprints: Vec<SourceFingerprint>,
}

pub(super) fn canonical_root(root: &str) -> Result<String, SessionError> {
    let canonical = std::fs::canonicalize(root).map_err(|error| {
        SessionError::new(
            SessionErrorCode::ExternalSourceChange,
            format!("canonicalize session root {root:?}: {error}"),
        )
    })?;
    canonical.into_os_string().into_string().map_err(|_| {
        SessionError::new(
            SessionErrorCode::PinnedRootMismatch,
            "canonical session root is not UTF-8",
        )
    })
}

pub(super) fn snapshot(
    profile: ResourceProfile,
    root: &str,
    expected: Option<&str>,
    ledger: &mut BudgetLedger,
) -> Result<SourceSnapshot, SessionError> {
    let request = Request {
        schema: crate::semantic::SCHEMA.to_string(),
        version: crate::semantic::VERSION,
        profile,
        root: root.to_string(),
        operation: OperationRequest::Snapshot {
            expected_repository_identity: expected.map(str::to_string),
        },
    };
    let request_bytes = crate::semantic::codec::measure_json(&request)
        .map_err(|error| SessionError::new(SessionErrorCode::ResourceLimit, error.message))?;
    let request_charge = u64::try_from(request_bytes).map_err(|_| {
        SessionError::new(
            SessionErrorCode::ResourceLimit,
            "source probe byte count overflow",
        )
    })?;
    crate::semantic::codec::reserve_request_bytes(ledger, request_charge)
        .map_err(|error| SessionError::new(SessionErrorCode::ResourceLimit, error.message))?;
    let outcome =
        crate::semantic::engine::execute_request_with_ledger(request, request_bytes, ledger)
            .map_err(|error| {
                SessionError::new(
                    SessionErrorCode::ExternalSourceChange,
                    format!("source revision probe failed: {}", error.message),
                )
            })?;
    let response = outcome.prepared.response;
    let revision = response.revision.clone().ok_or_else(|| {
        SessionError::new(
            SessionErrorCode::ExternalSourceChange,
            "source revision probe returned no revision",
        )
    })?;
    let fingerprints = match &response.result {
        ResponseResult::Snapshot { snapshot } => snapshot
            .source_units
            .iter()
            .map(SourceFingerprint::from)
            .collect(),
        ResponseResult::Error { error, .. } => {
            return Err(SessionError::new(
                SessionErrorCode::ExternalSourceChange,
                format!("source revision probe rejected: {}", error.message),
            ))
        }
        _ => {
            return Err(SessionError::new(
                SessionErrorCode::ExternalSourceChange,
                "source revision probe returned a non-snapshot response",
            ))
        }
    };
    Ok(SourceSnapshot {
        response,
        revision,
        fingerprints,
    })
}

pub(super) fn roots_match(pinned: &str, requested: &str) -> Result<bool, SessionError> {
    canonical_root(requested).map(|canonical| Path::new(pinned) == Path::new(&canonical))
}

pub(super) fn session_identity(
    compiler_build: &str,
    profile: ResourceProfile,
    root: &str,
    source_revision: &str,
) -> Result<String, SessionError> {
    let fields = (
        compiler_build,
        crate::semantic::SCHEMA,
        crate::semantic::VERSION,
        profile.core().name().as_str(),
        root,
        source_revision,
    );
    let encoded = serde_json::to_vec(&fields).map_err(|error| {
        SessionError::new(
            SessionErrorCode::ResourceLimit,
            format!("encode session identity: {error}"),
        )
    })?;
    Ok(crate::semantic::tree::hex(&lkjscript_core::sha256(
        &encoded,
    )))
}
