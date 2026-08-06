use crate::semantic::schema::{ApplyMode, OperationRequest, Request, ResponseResult};

use super::pending::PendingExecution;
use super::schema::{SessionError, SessionErrorCode, SessionResult};
use super::SemanticSession;

impl SemanticSession {
    pub(super) fn execute(&mut self, request: Request) -> SessionResult {
        let initialized = self.pinned.is_some();
        if !initialized {
            if let Err(error) = self.initialize(&request) {
                return SessionResult::Error { error };
            }
        }
        if let Err(error) = self.check_selection(&request) {
            return SessionResult::Error { error };
        }
        if initialized {
            let Some(pinned) = self.pinned.as_ref() else {
                return SessionResult::Error {
                    error: SessionError::new(
                        SessionErrorCode::NotInitialized,
                        "session lost its initialized pin",
                    ),
                };
            };
            let (root, expected) = (
                pinned.state.canonical_root.clone(),
                pinned.state.source_revision.clone(),
            );
            if let Err(error) = super::source::snapshot(&root, Some(&expected)) {
                return SessionResult::Error { error };
            }
        }
        let publishes = matches!(
            request.operation,
            OperationRequest::ApplyTransaction {
                mode: ApplyMode::Publish,
                ..
            }
        );
        if publishes {
            if let Err(error) = self.reserve_publication() {
                return SessionResult::Error { error };
            }
        }
        let request_bytes = match crate::semantic::codec::measure_json(&request) {
            Ok(bytes) => bytes,
            Err(error) => {
                return SessionResult::Error {
                    error: SessionError::new(SessionErrorCode::ResourceLimit, error.message),
                }
            }
        };
        if request_bytes > crate::semantic::MAX_REQUEST_BYTES {
            return SessionResult::Error {
                error: SessionError::new(
                    SessionErrorCode::ResourceLimit,
                    "typed request exceeds Semantic Source input byte policy",
                ),
            };
        }
        let outcome = match crate::semantic::engine::execute_request(request, request_bytes) {
            Ok(outcome) => outcome,
            Err(error) => {
                return SessionResult::Error {
                    error: SessionError::new(SessionErrorCode::ResourceLimit, error.message),
                }
            }
        };
        let response = outcome.prepared.response.clone();
        let rollback =
            if publishes && matches!(&response.result, ResponseResult::ApplyTransaction { .. }) {
                let Some(pinned) = self.pinned.clone() else {
                    return SessionResult::Error {
                        error: SessionError::new(
                            SessionErrorCode::NotInitialized,
                            "session lost its initialized pin",
                        ),
                    };
                };
                let rollback = (pinned, self.revision);
                if let Err(error) = self.accept_publication(&response) {
                    return SessionResult::Error { error };
                }
                Some(rollback)
            } else {
                None
            };
        let Some(pinned) = self.pinned.as_ref() else {
            return SessionResult::Error {
                error: SessionError::new(
                    SessionErrorCode::NotInitialized,
                    "session lost its initialized pin",
                ),
            };
        };
        let session = pinned.state.clone();
        self.pending = Some(PendingExecution::new(outcome, rollback));
        SessionResult::Execute {
            response: Box::new(response),
            session,
        }
    }

    fn check_selection(&self, request: &Request) -> Result<(), SessionError> {
        let pinned = self.pinned.as_ref().ok_or_else(|| {
            SessionError::new(
                SessionErrorCode::NotInitialized,
                "session is not initialized",
            )
        })?;
        match super::source::roots_match(&pinned.state.canonical_root, &request.root) {
            Ok(true) => Ok(()),
            Ok(false) => Err(SessionError::new(
                SessionErrorCode::PinnedRootMismatch,
                "semantic request attempted to repin the canonical root",
            )),
            Err(error) => Err(error),
        }
    }
}
