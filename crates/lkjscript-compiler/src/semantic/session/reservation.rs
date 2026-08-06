use super::limits::{
    MAX_SESSION_CUMULATIVE_OUTPUT_BYTES, MAX_SESSION_FRAME_BYTES,
    MAX_SESSION_RETAINED_METADATA_BYTES,
};
use super::schema::{SessionError, SessionErrorCode};
use super::SemanticSession;

impl SemanticSession {
    pub(super) fn reserve_publication(&self) -> Result<(), SessionError> {
        self.reserve_revision_response()
    }

    pub(super) fn reserve_revision_response(&self) -> Result<(), SessionError> {
        if self.pinned.is_none() {
            return Err(SessionError::new(
                SessionErrorCode::NotInitialized,
                "session is not initialized",
            ));
        }
        self.revision.checked_add(1).ok_or_else(|| {
            SessionError::new(
                SessionErrorCode::RevisionOverflow,
                "session revision overflow",
            )
        })?;
        let reserve = MAX_SESSION_FRAME_BYTES.checked_add(8).ok_or_else(|| {
            SessionError::new(
                SessionErrorCode::ResourceLimit,
                "output reservation overflow",
            )
        })?;
        let next = self.output_bytes.checked_add(reserve).ok_or_else(|| {
            SessionError::new(
                SessionErrorCode::ResourceLimit,
                "output reservation overflow",
            )
        })?;
        if next > MAX_SESSION_CUMULATIVE_OUTPUT_BYTES {
            return Err(SessionError::new(
                SessionErrorCode::ResourceLimit,
                "insufficient output bytes reserved for revision change",
            ));
        }
        Ok(())
    }

    pub(super) fn check_metadata(&self) -> Result<(), SessionError> {
        let Some(pinned) = self.pinned.as_ref() else {
            return Ok(());
        };
        let bytes = pinned
            .metadata_bytes()
            .and_then(|bytes| bytes.checked_add(8));
        if bytes.is_none_or(|bytes| bytes > MAX_SESSION_RETAINED_METADATA_BYTES) {
            return Err(SessionError::new(
                SessionErrorCode::ResourceLimit,
                "session retained metadata byte limit exceeded",
            ));
        }
        Ok(())
    }
}
