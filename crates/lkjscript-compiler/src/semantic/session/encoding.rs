use lkjscript_core::{BudgetCause, ResourceCategory};

use super::schema::{
    ProcessCode, SessionError, SessionErrorCode, SessionProcessError, SessionResponse,
    SessionResult,
};
use super::SemanticSession;

impl SemanticSession {
    pub(super) fn encode_result(
        &mut self,
        request_id: String,
        result: SessionResult,
    ) -> Result<Vec<u8>, SessionProcessError> {
        let envelope = SessionResponse {
            schema: super::SCHEMA,
            version: super::VERSION,
            request_id: request_id.clone(),
            revision: self.revision,
            response: result,
        };
        if let Some(encoded) = self.encode_envelope(&envelope)? {
            return Ok(encoded);
        }
        self.discard_pending();
        let fallback = SessionResponse {
            schema: super::SCHEMA,
            version: super::VERSION,
            request_id,
            revision: self.revision,
            response: SessionResult::Error {
                error: SessionError::new(
                    SessionErrorCode::ResourceLimit,
                    "session output limit exceeded",
                ),
            },
        };
        self.encode_envelope(&fallback)?.ok_or_else(|| {
            SessionProcessError::new(
                ProcessCode::FrameTooLarge,
                "session cannot frame a bounded error response",
            )
        })
    }

    fn encode_envelope(
        &mut self,
        envelope: &SessionResponse,
    ) -> Result<Option<Vec<u8>>, SessionProcessError> {
        let measured = crate::semantic::codec::measure_json(envelope)
            .map_err(|error| SessionProcessError::new(ProcessCode::OutputFailure, error.message))?;
        if !self.output_fits(measured) {
            return Ok(None);
        }
        let total = u64::try_from(measured)
            .ok()
            .and_then(|bytes| bytes.checked_add(8))
            .ok_or_else(|| {
                SessionProcessError::new(
                    ProcessCode::LengthOverflow,
                    "session output byte overflow",
                )
            })?;
        if let Some(ledger) = self.ledger.as_mut() {
            if super::reserve_session(
                ledger,
                ResourceCategory::SemanticSessionOutputBytes,
                total,
                BudgetCause::ProtocolFrame(total),
            )
            .is_err()
            {
                return Ok(None);
            }
        }
        let mut encoded = Vec::new();
        encoded.try_reserve_exact(measured).map_err(|error| {
            SessionProcessError::new(
                ProcessCode::OutputFailure,
                format!("reserve session response: {error}"),
            )
        })?;
        serde_json::to_writer(&mut encoded, envelope).map_err(|error| {
            SessionProcessError::new(
                ProcessCode::OutputFailure,
                format!("encode session response: {error}"),
            )
        })?;
        Ok(Some(encoded))
    }

    fn output_fits(&self, payload: usize) -> bool {
        let Ok(payload) = u64::try_from(payload) else {
            return false;
        };
        let Some(total) = payload.checked_add(8) else {
            return false;
        };
        if payload > self.frame_output_limit() {
            return false;
        }
        let limit = self.pinned.as_ref().map_or(
            super::limits::MAX_SESSION_CUMULATIVE_OUTPUT_BYTES,
            |pinned| pinned.state.limits.cumulative_output_bytes,
        );
        self.output_bytes
            .checked_add(total)
            .is_some_and(|next| next <= limit)
    }
}
