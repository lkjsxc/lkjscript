use crate::*;

impl JitSession {
    pub(crate) fn snapshot_reference_return(
        &self,
        function: FunctionId,
        reference: lkjscript_native::NativeReference,
    ) -> Result<OwnedValue, EngineError> {
        let lists = self.lists.as_ref().ok_or_else(|| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                "reference return lacks segmented-list arena",
            )
        })?;
        if matches!(
            reference.reference_type(),
            ReferenceType::RegionProduct(_, _)
        ) {
            return Err(invocation_failure(
                function,
                "invocation-region product cannot cross the process boundary",
            ));
        }
        if !matches!(reference.reference_type(), ReferenceType::List(_, _, _, _)) {
            return Err(invocation_failure(
                function,
                "non-list reference cannot cross the process boundary",
            ));
        }
        let root = {
            let key = lists
                .key_from_word(reference.opaque_word())
                .map_err(|error| {
                    invocation_failure(
                        function,
                        format!("invalid segmented-list return: {error:?}"),
                    )
                })?;
            lists
                .validate_type(key, reference_layout_key(reference.reference_type()))
                .map_err(|error| {
                    invocation_failure(
                        function,
                        format!("segmented-list return type mismatch: {error:?}"),
                    )
                })?;
            if key.is_empty() {
                Value::EMPTY_LIST
            } else {
                Value::from_segmented_list(key.to_word())
            }
        };
        let limit = lists.limits().max_entries.get() as usize;
        OwnedValue::from_segmented_list_snapshot(root, limit, |word| {
            let key = lists.key_from_word(word).map_err(|error| {
                lkjscript_core::Error::msg(format!(
                    "invalid nested segmented-list return: {error:?}"
                ))
            })?;
            lists.collect_cloned(key, u32::MAX).map_err(|error| {
                lkjscript_core::Error::msg(format!(
                    "nested segmented-list snapshot failed: {error:?}"
                ))
            })
        })
        .map_err(|error| invocation_failure(function, error.to_string()))
    }
}

fn invocation_failure(function: FunctionId, message: impl Into<String>) -> EngineError {
    EngineError::new(
        FailureCode::InvocationFailure,
        Some(function),
        message.into(),
    )
}
