use crate::semantic::charges::ProtocolLimits;

use super::limits::MAX_SESSION_RETAINED_METADATA_BYTES;
use super::schema::{SessionError, SessionErrorCode};
use super::SemanticSession;

impl SemanticSession {
    pub(super) fn reserve_publication(&self) -> Result<(), SessionError> {
        self.reserve_revision_response()?;
        let Some(pinned) = self.pinned.as_ref() else {
            return Err(SessionError::new(
                SessionErrorCode::NotInitialized,
                "publication requires an initialized session",
            ));
        };
        let request_work = ProtocolLimits::for_profile(pinned.state.profile).work_units;
        let remaining = pinned
            .state
            .limits
            .lifetime_fuel
            .saturating_sub(self.lifetime_fuel);
        if remaining < request_work {
            return Err(SessionError::new(
                SessionErrorCode::ResourceLimit,
                "insufficient session fuel reserved for atomic publication",
            ));
        }
        Ok(())
    }

    pub(super) fn reserve_revision_response(&self) -> Result<(), SessionError> {
        let Some(pinned) = self.pinned.as_ref() else {
            return Err(SessionError::new(
                SessionErrorCode::NotInitialized,
                "session is not initialized",
            ));
        };
        if self.revision >= pinned.state.limits.maximum_revision {
            return Err(SessionError::new(
                SessionErrorCode::RevisionOverflow,
                "session revision maximum reached",
            ));
        }
        let reserve = pinned
            .state
            .limits
            .frame_output_bytes
            .checked_add(8)
            .ok_or_else(|| {
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
        if next > pinned.state.limits.cumulative_output_bytes {
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
        let limit = pinned
            .state
            .limits
            .retained_metadata_bytes
            .min(MAX_SESSION_RETAINED_METADATA_BYTES);
        let bytes = pinned
            .metadata_bytes()
            .and_then(|bytes| bytes.checked_add(8));
        if bytes.is_none_or(|bytes| bytes > limit) {
            return Err(SessionError::new(
                SessionErrorCode::ResourceLimit,
                "session retained metadata limit exceeded",
            ));
        }
        Ok(())
    }
}
