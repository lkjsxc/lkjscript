use crate::semantic::session::limits::{MAX_REQUEST_ID_BYTES, MAX_SESSION_REQUESTS};
use lkjscript_core::BudgetLedger;

use super::schema::{
    ProcessCode, SessionError, SessionErrorCode, SessionOperation, SessionProcessError,
    SessionRequest, SessionResult,
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
        if let Some(ledger) = self.ledger.as_mut() {
            ledger.rollover_request_segment();
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
        if let SessionOperation::Execute { request } = &envelope.request {
            if let Err(error) = self.ensure_ledger(request.profile) {
                return self.encode_result(envelope.request_id, SessionResult::Error { error });
            }
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

    fn ensure_ledger(
        &mut self,
        profile: crate::semantic::schema::ResourceProfile,
    ) -> Result<(), SessionError> {
        match &self.ledger {
            Some(ledger) if crate::semantic::budget::profile_matches(profile, ledger) => Ok(()),
            Some(_) => Err(SessionError::new(
                SessionErrorCode::PinnedProfileMismatch,
                "session request profile does not match outer-owned ledger",
            )),
            None => {
                self.ledger = Some(BudgetLedger::new(profile.core()));
                Ok(())
            }
        }
    }
}
