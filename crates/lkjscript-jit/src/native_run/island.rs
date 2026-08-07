use crate::*;

impl NativeRun {
    pub(crate) fn take_returned_unique(
        &mut self,
        function: FunctionId,
        bytes: bool,
    ) -> Result<OwnedValue, EngineError> {
        let payload = self.returned_unique.take().ok_or_else(|| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                "native unique return has no transferred backing",
            )
        })?;
        let result = if bytes {
            OwnedValue::from_unique_bytes(payload)
        } else {
            OwnedValue::from_unique_byte_vector(payload)
        };
        result.map_err(|error| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                error.to_string(),
            )
        })
    }

    pub fn take_returned_structural(
        &mut self,
        function: FunctionId,
    ) -> Result<SemanticValue, EngineError> {
        self.returned_structural
            .take()
            .map(|returned| returned.0)
            .ok_or_else(|| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "native structural return has no exported semantic tree",
                )
            })
    }
}
