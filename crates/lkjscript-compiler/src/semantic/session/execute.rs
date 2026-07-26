use crate::semantic::schema::{ApplyMode, OperationRequest, Request, Response, ResponseResult};

use super::pending::PendingExecution;
use super::schema::{SessionError, SessionErrorCode, SessionResult};
use super::SemanticSession;

fn response_fuel(response: &Response) -> Result<u64, SessionError> {
    response
        .charges
        .work_units
        .checked_add(response.charges.hole_search_work)
        .and_then(|value| value.checked_add(response.charges.legal_actions))
        .and_then(|value| value.checked_add(response.charges.transaction_impact_nodes))
        .ok_or_else(|| {
            SessionError::new(
                SessionErrorCode::ResourceLimit,
                "semantic session lifetime fuel charge overflow",
            )
        })
}

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
            let (profile, root, expected) = (
                pinned.state.profile,
                pinned.state.canonical_root.clone(),
                pinned.state.source_revision.clone(),
            );
            let Some(ledger) = self.ledger.as_mut() else {
                return SessionResult::Error {
                    error: SessionError::new(
                        SessionErrorCode::ResourceLimit,
                        "session ledger authority is missing",
                    ),
                };
            };
            let snapshot = match super::source::snapshot(profile, &root, Some(&expected), ledger) {
                Ok(snapshot) => snapshot,
                Err(error) => return SessionResult::Error { error },
            };
            if let Err(error) = self.charge_fuel(snapshot.response.charges.work_units) {
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
        let Some(ledger) = self.ledger.as_mut() else {
            return SessionResult::Error {
                error: SessionError::new(
                    SessionErrorCode::ResourceLimit,
                    "session ledger authority is missing",
                ),
            };
        };
        let request_charge = match u64::try_from(request_bytes) {
            Ok(bytes) => bytes,
            Err(_) => {
                return SessionResult::Error {
                    error: SessionError::new(
                        SessionErrorCode::ResourceLimit,
                        "typed request byte count overflow",
                    ),
                }
            }
        };
        if let Err(error) = crate::semantic::codec::reserve_request_bytes(ledger, request_charge) {
            return SessionResult::Error {
                error: SessionError::new(SessionErrorCode::ResourceLimit, error.message),
            };
        }
        let outcome = match crate::semantic::engine::execute_request_with_ledger(
            request,
            request_bytes,
            ledger,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                return SessionResult::Error {
                    error: SessionError::new(SessionErrorCode::ResourceLimit, error.message),
                }
            }
        };
        let response = outcome.prepared.response.clone();
        let fuel = response_fuel(&response);
        let fuel = match fuel {
            Ok(fuel) => fuel,
            Err(error) => return SessionResult::Error { error },
        };
        if let Err(error) = self.charge_fuel(fuel) {
            return SessionResult::Error { error };
        }
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
        if request.profile != pinned.state.profile {
            return Err(SessionError::new(
                SessionErrorCode::PinnedProfileMismatch,
                "semantic request attempted to repin the resource profile",
            ));
        }
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
