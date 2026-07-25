use crate::semantic::session::limits::{MAX_REQUEST_ID_BYTES, MAX_SESSION_REQUESTS};

use super::schema::{
    ProcessCode, SessionError, SessionErrorCode, SessionOperation, SessionProcessError,
    SessionRequest, SessionResponse, SessionResult,
};
use super::SemanticSession;

impl SemanticSession {
    pub(super) fn handle(&mut self, input: &[u8]) -> Result<Vec<u8>, SessionProcessError> {
        std::str::from_utf8(input).map_err(|_| {
            SessionProcessError::new(ProcessCode::InvalidJson, "session request is not UTF-8")
        })?;
        let envelope: SessionRequest = serde_json::from_slice(input).map_err(|error| {
            SessionProcessError::new(
                ProcessCode::InvalidJson,
                format!("strict session JSON rejected: {error}"),
            )
        })?;
        if envelope.schema != super::SCHEMA {
            return Err(SessionProcessError::new(
                ProcessCode::InvalidJson,
                format!("unknown session schema {:?}", envelope.schema),
            ));
        }
        if envelope.version != super::VERSION {
            return Err(SessionProcessError::new(
                ProcessCode::InvalidJson,
                format!("unsupported session version {}", envelope.version),
            ));
        }
        if envelope.request_id.len() > MAX_REQUEST_ID_BYTES {
            return self.encode_result(
                envelope.request_id,
                SessionResult::Error {
                    error: SessionError::new(
                        SessionErrorCode::ResourceLimit,
                        "session request_id exceeds 256 bytes",
                    ),
                },
            );
        }
        let request_limit = self.pinned.as_ref().map_or(MAX_SESSION_REQUESTS, |pinned| {
            pinned.state.limits.request_count
        });
        let next_requests = self.requests.checked_add(1);
        if next_requests.is_none_or(|requests| requests > request_limit) {
            return self.encode_result(
                envelope.request_id,
                SessionResult::Error {
                    error: SessionError::new(
                        SessionErrorCode::ResourceLimit,
                        "session request-count limit exceeded",
                    ),
                },
            );
        }
        self.requests = next_requests.unwrap_or(self.requests);
        if envelope.revision != self.revision {
            return self.encode_result(
                envelope.request_id,
                SessionResult::Error {
                    error: SessionError::new(
                        SessionErrorCode::StaleSessionRevision,
                        format!(
                            "expected session revision {}, received {}",
                            self.revision, envelope.revision
                        ),
                    ),
                },
            );
        }
        let result = match envelope.request {
            SessionOperation::Execute { request } => self.execute(request),
            SessionOperation::Refresh => self.refresh(),
            SessionOperation::Shutdown => {
                self.closed = true;
                SessionResult::Shutdown { acknowledged: true }
            }
        };
        self.encode_result(envelope.request_id, result)
    }

    fn encode_result(
        &self,
        request_id: String,
        result: SessionResult,
    ) -> Result<Vec<u8>, SessionProcessError> {
        let envelope = SessionResponse {
            schema: super::SCHEMA,
            version: super::VERSION,
            request_id: request_id.clone(),
            revision: self.revision,
            response: result,
        };
        let encoded = serde_json::to_vec(&envelope).map_err(|error| {
            SessionProcessError::new(
                ProcessCode::OutputFailure,
                format!("encode session response: {error}"),
            )
        })?;
        if self.output_fits(encoded.len()) {
            return Ok(encoded);
        }
        let fallback = SessionResponse {
            schema: super::SCHEMA,
            version: super::VERSION,
            request_id,
            revision: self.revision,
            response: SessionResult::Error {
                error: SessionError::new(
                    SessionErrorCode::ResourceLimit,
                    "session output limit exceeded",
                ),
            },
        };
        let encoded = serde_json::to_vec(&fallback).map_err(|error| {
            SessionProcessError::new(ProcessCode::OutputFailure, error.to_string())
        })?;
        if self.output_fits(encoded.len()) {
            Ok(encoded)
        } else {
            Err(SessionProcessError::new(
                ProcessCode::FrameTooLarge,
                "session cannot frame a bounded error response",
            ))
        }
    }

    fn output_fits(&self, payload: usize) -> bool {
        let Ok(payload) = u64::try_from(payload) else {
            return false;
        };
        let Some(total) = payload.checked_add(8) else {
            return false;
        };
        if payload > self.frame_output_limit() {
            return false;
        }
        let limit = self.pinned.as_ref().map_or(
            super::limits::MAX_SESSION_CUMULATIVE_OUTPUT_BYTES,
            |pinned| pinned.state.limits.cumulative_output_bytes,
        );
        self.output_bytes
            .checked_add(total)
            .is_some_and(|next| next <= limit)
    }
}
