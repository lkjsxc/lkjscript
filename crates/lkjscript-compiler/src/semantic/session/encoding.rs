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
            contract: super::CONTRACT.to_hex(),
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
            contract: super::CONTRACT.to_hex(),
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
        let mut encoded = Vec::new();
        encoded.try_reserve_exact(measured).map_err(|error| {
            SessionProcessError::new(
                ProcessCode::OutputFailure,
                format!("reserve session response: {error}"),
            )
        })?;
        crate::semantic::codec::write_json(&mut encoded, envelope).map_err(|error| {
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
        let limit = super::limits::MAX_SESSION_CUMULATIVE_OUTPUT_BYTES;
        self.output_bytes
            .checked_add(total)
            .is_some_and(|next| next <= limit)
    }
}
