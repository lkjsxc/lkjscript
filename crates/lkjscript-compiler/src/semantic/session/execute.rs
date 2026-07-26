use crate::semantic::schema::{ApplyMode, OperationRequest, Request, Response, ResponseResult};

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
            let snapshot = match super::source::snapshot(profile, &root, Some(&expected)) {
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
        let encoded = match serde_json::to_vec(&request) {
            Ok(encoded) => encoded,
            Err(error) => {
                return SessionResult::Error {
                    error: SessionError::new(
                        SessionErrorCode::ResourceLimit,
                        format!("encode semantic request: {error}"),
                    ),
                }
            }
        };
        let output = match crate::semantic::execute(&encoded) {
            Ok(output) => output,
            Err(error) => {
                return SessionResult::Error {
                    error: SessionError::new(
                        SessionErrorCode::ResourceLimit,
                        format!("one-shot semantic engine rejected typed request: {error}"),
                    ),
                }
            }
        };
        let response: Response = match serde_json::from_slice(&output) {
            Ok(response) => response,
            Err(error) => {
                return SessionResult::Error {
                    error: SessionError::new(
                        SessionErrorCode::ResourceLimit,
                        format!("decode one-shot semantic response: {error}"),
                    ),
                }
            }
        };
        let fuel = response_fuel(&response);
        let fuel = match fuel {
            Ok(fuel) => fuel,
            Err(error) => return SessionResult::Error { error },
        };
        if let Err(error) = self.charge_fuel(fuel) {
            return SessionResult::Error { error };
        }
        if publishes && matches!(&response.result, ResponseResult::ApplyTransaction { .. }) {
            if let Err(error) = self.accept_publication(&response) {
                return SessionResult::Error { error };
            }
        }
        let Some(pinned) = self.pinned.as_ref() else {
            return SessionResult::Error {
                error: SessionError::new(
                    SessionErrorCode::NotInitialized,
                    "session lost its initialized pin",
                ),
            };
        };
        let session = pinned.state.clone();
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

    fn accept_publication(&mut self, response: &Response) -> Result<(), SessionError> {
        let revision = response.revision.clone().ok_or_else(|| {
            SessionError::new(
                SessionErrorCode::ExternalSourceChange,
                "published transaction returned no source revision",
            )
        })?;
        let Some(pinned) = self.pinned.as_mut() else {
            return Err(SessionError::new(
                SessionErrorCode::NotInitialized,
                "published session lost its initialized pin",
            ));
        };
        let ResponseResult::ApplyTransaction { transaction } = &response.result else {
            return Ok(());
        };
        for change in &transaction.changed_sources {
            let fingerprint = pinned
                .fingerprints
                .iter_mut()
                .find(|fingerprint| fingerprint.path == change.path)
                .ok_or_else(|| {
                    SessionError::new(
                        SessionErrorCode::ExternalSourceChange,
                        "published source was outside the pinned fingerprint set",
                    )
                })?;
            fingerprint.bytes = change.bytes;
            fingerprint.sha256.clone_from(&change.new_sha256);
        }
        pinned.state.source_revision = revision;
        self.check_metadata()?;
        self.advance_revision()
    }
}
