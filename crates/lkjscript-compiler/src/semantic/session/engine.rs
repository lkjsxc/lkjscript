use serde::Deserialize;

use crate::semantic::session::limits::{MAX_REQUEST_ID_BYTES, MAX_SESSION_REQUESTS};

use super::schema::{
    ProcessCode, SessionError, SessionErrorCode, SessionOperation, SessionProcessError,
    SessionRequest, SessionResult,
};
use super::SemanticSession;

fn session_json_error(error: serde_json::Error) -> SessionProcessError {
    SessionProcessError::new(
        ProcessCode::InvalidJson,
        format!("strict session JSON rejected: {error}"),
    )
}

impl SemanticSession {
    pub(super) fn handle(&mut self, input: &[u8]) -> Result<Vec<u8>, SessionProcessError> {
        std::str::from_utf8(input).map_err(|_| {
            SessionProcessError::new(ProcessCode::InvalidJson, "session request is not UTF-8")
        })?;
        let mut deserializer = serde_json::Deserializer::from_slice(input);
        deserializer.disable_recursion_limit();
        let mut stacked = serde_stacker::Deserializer::new(&mut deserializer);
        stacked.red_zone = crate::semantic::codec::SERDE_STACK_RED_ZONE_BYTES;
        let envelope = SessionRequest::deserialize(stacked).map_err(session_json_error)?;
        deserializer.end().map_err(session_json_error)?;
        if envelope.schema != super::SCHEMA {
            return Err(SessionProcessError::new(
                ProcessCode::InvalidJson,
                format!("unknown session schema {:?}", envelope.schema),
            ));
        }
        if lkjscript_contracts::ContractDigest::from_hex(&envelope.contract)
            != Some(super::CONTRACT)
        {
            return Err(SessionProcessError::new(
                ProcessCode::InvalidJson,
                format!(
                    concat!(
                        "contract mismatch for {}: expected {}, actual {}; ",
                        "producer=session request, consumer=lkjscript compiler; update the producer"
                    ),
                    super::SCHEMA,
                    super::CONTRACT,
                    envelope.contract,
                ),
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
        let next_requests = self.requests.checked_add(1);
        if next_requests.is_none_or(|requests| requests > MAX_SESSION_REQUESTS) {
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
        let request_id = envelope.request_id;
        let encoded = match self.encode_result(request_id.clone(), result) {
            Ok(encoded) => encoded,
            Err(error) => {
                self.discard_pending();
                return Err(error);
            }
        };
        if let Err(error) = self.publish_pending() {
            return self.encode_result(
                request_id,
                SessionResult::Error {
                    error: SessionError::new(SessionErrorCode::ResourceLimit, error.message),
                },
            );
        }
        Ok(encoded)
    }
}
