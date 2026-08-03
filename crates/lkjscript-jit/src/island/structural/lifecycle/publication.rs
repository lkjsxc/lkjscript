use super::*;

impl JitStructuralRuntime {
    pub(in crate::island::structural) fn publish_static(
        &mut self,
        bytes: &[u8],
        value_type: StructuralTypeIdentity,
        kind: StructuralPayloadKind,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let expected = core_type(value_type)?;
        let mut owned = Vec::new();
        if owned.try_reserve_exact(bytes.len()).is_err() {
            self.last_resource = Some(ResourceLimitKind::HeapBytes);
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        owned.extend_from_slice(bytes);
        self.publish_semantic(
            SemanticValue::new(expected, payload(owned, kind)),
            value_type,
            storage,
        )
    }

    pub(in crate::island::structural) fn publish_unique(
        &mut self,
        bytes: Vec<u8>,
        value_type: StructuralTypeIdentity,
        kind: StructuralPayloadKind,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let expected = core_type(value_type)?;
        self.publish_semantic(
            SemanticValue::new(expected, payload(bytes, kind)),
            value_type,
            storage,
        )
    }

    pub(in crate::island::structural) fn publish_i64(
        &mut self,
        value: i64,
        value_type: StructuralTypeIdentity,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let expected = core_type(value_type)?;
        self.publish_semantic(
            SemanticValue::new(
                expected,
                SemanticPayload::Inline(InlineStructuralValue::I64(value)),
            ),
            value_type,
            storage,
        )
    }

    fn publish_semantic(
        &mut self,
        value: SemanticValue,
        value_type: StructuralTypeIdentity,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        let expected = core_type(value_type)?;
        let unique = self
            .runtime
            .publish_owned(value)
            .map_err(|failure| self.map_error(failure.error))?;
        let key = match storage {
            StructuralStorageRoute::Unique => unique,
            StructuralStorageRoute::Sealed => match self.runtime.seal_owned(unique, expected) {
                Ok(sealed) => sealed.owner,
                Err(error) => {
                    let _ = self.runtime.dispose_owner(unique, expected);
                    return Err(self.map_error(error));
                }
            },
        };
        self.register_runtime_owner(key, expected, value_type, storage)?;
        Ok(NativeStructuralOwner::new(value_type, key.get()))
    }

    pub(in crate::island::structural) fn publish_formatted_i64(
        &mut self,
        value: i64,
        value_type: StructuralTypeIdentity,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let expected = core_type(value_type)?;
        self.publish_semantic(
            SemanticValue::new(
                expected,
                SemanticPayload::String(value.to_string().into_bytes()),
            ),
            value_type,
            StructuralStorageRoute::Unique,
        )
    }
}
