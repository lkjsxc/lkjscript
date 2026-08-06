use crate::semantic::schema::Request;

use super::schema::{
    PinnedSession, SessionError, SessionErrorCode, SessionResult, SessionStateRecord,
};
use super::SemanticSession;

impl SemanticSession {
    pub(super) fn initialize(&mut self, request: &Request) -> Result<(), SessionError> {
        let root = super::source::canonical_root(&request.root)?;
        let snapshot = super::source::snapshot(&root, None)?;
        let session_identity = super::source::session_identity(
            &snapshot.response.compiler_build,
            &root,
            &snapshot.revision,
        )?;
        let state = SessionStateRecord {
            session_identity,
            compiler_build: snapshot.response.compiler_build.clone(),
            semantic_schema: crate::semantic::SCHEMA.to_string(),
            semantic_contract: crate::semantic::CONTRACT.to_hex(),
            diagnostic_schema: lkjscript_contracts::DIAGNOSTICS.to_string(),
            diagnostic_contract: lkjscript_contracts::DIAGNOSTICS_DIGEST.to_hex(),
            canonical_root: root,
            source_revision: snapshot.revision,
            cache_entries: 0,
        };
        self.pinned = Some(PinnedSession {
            state,
            fingerprints: snapshot.fingerprints,
        });
        if let Err(error) = self.check_metadata().and_then(|()| self.advance_revision()) {
            self.pinned = None;
            self.revision = 0;
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
        let root = pinned.state.canonical_root.clone();
        let snapshot = match super::source::snapshot(&root, None) {
            Ok(snapshot) => snapshot,
            Err(error) => return SessionResult::Error { error },
        };
        let previous = self.pinned.clone();
        let previous_revision = self.revision;
        let Some(pinned) = self.pinned.as_mut() else {
            return SessionResult::Error {
                error: SessionError::new(SessionErrorCode::NotInitialized, "session lost pin"),
            };
        };
        pinned.state.source_revision = snapshot.revision;
        pinned.fingerprints = snapshot.fingerprints;
        if let Err(error) = self.check_metadata().and_then(|()| self.advance_revision()) {
            self.pinned = previous;
            self.revision = previous_revision;
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

    pub(super) fn advance_revision(&mut self) -> Result<(), SessionError> {
        self.revision = self.revision.checked_add(1).ok_or_else(|| {
            SessionError::new(
                SessionErrorCode::RevisionOverflow,
                "session revision overflow",
            )
        })?;
        Ok(())
    }
}
