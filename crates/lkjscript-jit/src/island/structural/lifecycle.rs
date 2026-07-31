use super::*;

impl JitStructuralRuntime {
    pub(super) fn publish_static(
        &mut self,
        bytes: &[u8],
        value_type: StructuralTypeIdentity,
        kind: StructuralPayloadKind,
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
        )
    }

    pub(super) fn publish_unique(
        &mut self,
        bytes: Vec<u8>,
        value_type: StructuralTypeIdentity,
        kind: StructuralPayloadKind,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let expected = core_type(value_type)?;
        self.publish_semantic(
            SemanticValue::new(expected, payload(bytes, kind)),
            value_type,
        )
    }

    pub(super) fn publish_i64(
        &mut self,
        value: i64,
        value_type: StructuralTypeIdentity,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let expected = core_type(value_type)?;
        self.publish_semantic(
            SemanticValue::new(
                expected,
                SemanticPayload::Inline(InlineStructuralValue::I64(value)),
            ),
            value_type,
        )
    }

    fn publish_semantic(
        &mut self,
        value: SemanticValue,
        value_type: StructuralTypeIdentity,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        let key = self
            .runtime
            .publish_owned(value)
            .map_err(|failure| self.map_error(failure.error))?;
        self.owners.insert(key.get(), value_type);
        Ok(NativeStructuralOwner::new(value_type, key.get()))
    }

    pub(super) fn publish_formatted_i64(
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
        )
    }

    pub(super) fn capture_trap(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<(), NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let key = owner_key(owner)?;
        let semantic = self
            .runtime
            .export_semantic(key, expected)
            .map_err(|error| self.map_error(error))?;
        let SemanticPayload::String(bytes) = semantic.payload else {
            return Err(NativeServiceError::Trap);
        };
        let message = String::from_utf8(bytes).map_err(|_| NativeServiceError::Trap)?;
        self.owners.remove(&key.get());
        self.last_trap = Some(message);
        Ok(())
    }

    pub(super) fn copy_owner(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let copy = self
            .runtime
            .clone_owned(owner_key(owner)?, expected)
            .map_err(|error| self.map_error(error))?;
        self.owners.insert(copy.get(), owner.structural_type());
        Ok(NativeStructuralOwner::new(
            owner.structural_type(),
            copy.get(),
        ))
    }

    pub(super) fn move_owner(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let key = owner_key(owner)?;
        let next = self
            .runtime
            .move_owned(key, expected)
            .map_err(|error| self.map_error(error))?;
        self.owners.remove(&key.get());
        self.owners.insert(next.get(), owner.structural_type());
        Ok(NativeStructuralOwner::new(
            owner.structural_type(),
            next.get(),
        ))
    }

    pub(super) fn drop_owner(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<(), NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let key = owner_key(owner)?;
        self.runtime
            .drop_owned(key, expected)
            .map_err(|error| self.map_error(error))?;
        self.owners.remove(&key.get());
        Ok(())
    }
}
