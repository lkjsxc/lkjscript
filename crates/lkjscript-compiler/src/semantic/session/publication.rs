use crate::semantic::schema::{Response, ResponseResult};

use super::schema::{SessionError, SessionErrorCode};
use super::SemanticSession;

impl SemanticSession {
    pub(super) fn accept_publication(&mut self, response: &Response) -> Result<(), SessionError> {
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
