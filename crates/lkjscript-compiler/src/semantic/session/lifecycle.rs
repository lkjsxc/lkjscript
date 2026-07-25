use crate::semantic::schema::Request;

use super::limits::{SessionLimits, MAX_SESSION_LIFETIME_FUEL, MAX_SESSION_REVISION};
use super::schema::{
    PinnedSession, SessionError, SessionErrorCode, SessionResult, SessionStateRecord,
};
use super::SemanticSession;

impl SemanticSession {
    pub(super) fn initialize(&mut self, request: &Request) -> Result<(), SessionError> {
        let root = super::source::canonical_root(&request.root)?;
        let snapshot = super::source::snapshot(request.profile, &root, None)?;
        let limits = SessionLimits::for_profile(request.profile);
        if snapshot.response.charges.work_units > limits.lifetime_fuel {
            return Err(SessionError::new(
                SessionErrorCode::ResourceLimit,
                "initial source probe exceeds session lifetime fuel",
            ));
        }
        let session_identity = super::source::session_identity(
            &snapshot.response.compiler_build,
            request.profile,
            &root,
            &snapshot.revision,
        )?;
        let state = SessionStateRecord {
            session_identity,
            compiler_build: snapshot.response.compiler_build.clone(),
            semantic_schema: crate::semantic::SCHEMA.to_string(),
            semantic_version: crate::semantic::VERSION,
            diagnostic_schema: crate::source::SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA.to_string(),
            diagnostic_version: crate::source::SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA_VERSION,
            profile: request.profile,
            profile_identity: snapshot.response.profile_identity,
            canonical_root: root,
            source_revision: snapshot.revision,
            limits,
            cache_entries: 0,
        };
        self.pinned = Some(PinnedSession {
            state,
            fingerprints: snapshot.fingerprints,
        });
        self.lifetime_fuel = snapshot.response.charges.work_units;
        if let Err(error) = self.check_metadata().and_then(|()| self.advance_revision()) {
            self.pinned = None;
            self.lifetime_fuel = 0;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn refresh(&mut self) -> SessionResult {
        if let Err(error) = self.reserve_revision_response() {
            return SessionResult::Error { error };
        }
        let Some(pinned) = self.pinned.as_ref() else {
            return SessionResult::Error {
                error: SessionError::new(
                    SessionErrorCode::NotInitialized,
                    "refresh requires a successful execute request",
                ),
            };
        };
        let profile = pinned.state.profile;
        let root = pinned.state.canonical_root.clone();
        let snapshot = match super::source::snapshot(profile, &root, None) {
            Ok(snapshot) => snapshot,
            Err(error) => return SessionResult::Error { error },
        };
        if let Err(error) = self.charge_fuel(snapshot.response.charges.work_units) {
            return SessionResult::Error { error };
        }
        let Some(pinned) = self.pinned.as_mut() else {
            return SessionResult::Error {
                error: SessionError::new(SessionErrorCode::NotInitialized, "session lost pin"),
            };
        };
        pinned.state.source_revision = snapshot.revision;
        pinned.fingerprints = snapshot.fingerprints;
        if let Err(error) = self.check_metadata().and_then(|()| self.advance_revision()) {
            return SessionResult::Error { error };
        }
        let Some(pinned) = self.pinned.as_ref() else {
            return SessionResult::Error {
                error: SessionError::new(SessionErrorCode::NotInitialized, "session lost pin"),
            };
        };
        SessionResult::Refresh {
            session: pinned.state.clone(),
        }
    }

    pub(super) fn charge_fuel(&mut self, increment: u64) -> Result<(), SessionError> {
        let limit = self
            .pinned
            .as_ref()
            .map_or(MAX_SESSION_LIFETIME_FUEL, |pinned| {
                pinned.state.limits.lifetime_fuel
            });
        let next = self.lifetime_fuel.checked_add(increment).ok_or_else(|| {
            SessionError::new(
                SessionErrorCode::ResourceLimit,
                "session lifetime fuel overflow",
            )
        })?;
        if next > limit {
            return Err(SessionError::new(
                SessionErrorCode::ResourceLimit,
                format!("session lifetime fuel {next} exceeds {limit}"),
            ));
        }
        self.lifetime_fuel = next;
        Ok(())
    }

    pub(super) fn advance_revision(&mut self) -> Result<(), SessionError> {
        let maximum = self.pinned.as_ref().map_or(MAX_SESSION_REVISION, |pinned| {
            pinned.state.limits.maximum_revision
        });
        let next = self.revision.checked_add(1).ok_or_else(|| {
            SessionError::new(
                SessionErrorCode::RevisionOverflow,
                "session revision overflow",
            )
        })?;
        if next > maximum {
            return Err(SessionError::new(
                SessionErrorCode::RevisionOverflow,
                "session revision maximum reached",
            ));
        }
        self.revision = next;
        Ok(())
    }
}
