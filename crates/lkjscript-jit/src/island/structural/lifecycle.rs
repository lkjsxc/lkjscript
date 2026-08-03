use super::*;

mod publication;
mod registry;

impl JitStructuralRuntime {
    pub(super) fn capture_trap(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<(), NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let key = owner_key(owner)?;
        self.require_owner(owner, None)?;
        let semantic = self
            .runtime
            .export_semantic(key, expected)
            .map_err(|error| self.map_error(error))?;
        let SemanticPayload::String(bytes) = semantic.payload else {
            return Err(NativeServiceError::Trap);
        };
        let message = String::from_utf8(bytes).map_err(|_| NativeServiceError::Trap)?;
        self.last_trap = Some(message);
        self.owners.remove(&key.get());
        Ok(())
    }

    pub(super) fn publish_owner(
        &mut self,
        owner: NativeStructuralOwner,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let record = self.require_owner(owner, None)?;
        let expected = core_type(owner.structural_type())?;
        let key = owner_key(owner)?;
        let next = match (record.storage, storage) {
            (StructuralStorageRoute::Unique, StructuralStorageRoute::Unique) => self
                .runtime
                .move_owned(key, expected)
                .map_err(|error| self.map_error(error))?,
            (StructuralStorageRoute::Unique, StructuralStorageRoute::Sealed) => self
                .runtime
                .seal_owned(key, expected)
                .map(|sealed| sealed.owner)
                .map_err(|error| self.map_error(error))?,
            (StructuralStorageRoute::Sealed, StructuralStorageRoute::Sealed) => self
                .runtime
                .move_sealed(key, expected)
                .map_err(|error| self.map_error(error))?,
            (StructuralStorageRoute::Sealed, StructuralStorageRoute::Unique) => {
                return Err(NativeServiceError::Trap)
            }
        };
        self.replace_runtime_owner(key, next, expected, owner.structural_type(), storage)?;
        Ok(NativeStructuralOwner::new(
            owner.structural_type(),
            next.get(),
        ))
    }

    pub(super) fn copy_owner(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let record = self.require_owner(owner, None)?;
        let expected = core_type(owner.structural_type())?;
        let copy = match record.storage {
            StructuralStorageRoute::Unique => self
                .runtime
                .clone_owned(owner_key(owner)?, expected)
                .map_err(|error| self.map_error(error))?,
            StructuralStorageRoute::Sealed => self
                .runtime
                .acquire_sealed(owner_key(owner)?, expected)
                .map_err(|error| self.map_error(error))?,
        };
        self.register_runtime_owner(copy, expected, owner.structural_type(), record.storage)?;
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
        let record = self.require_owner(owner, None)?;
        let expected = core_type(owner.structural_type())?;
        let key = owner_key(owner)?;
        let next = match record.storage {
            StructuralStorageRoute::Unique => self
                .runtime
                .move_owned(key, expected)
                .map_err(|error| self.map_error(error))?,
            StructuralStorageRoute::Sealed => self
                .runtime
                .move_sealed(key, expected)
                .map_err(|error| self.map_error(error))?,
        };
        self.replace_runtime_owner(key, next, expected, owner.structural_type(), record.storage)?;
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
        self.require_owner(owner, None)?;
        let expected = core_type(owner.structural_type())?;
        let key = owner_key(owner)?;
        self.runtime
            .dispose_owner(key, expected)
            .map_err(|error| self.map_error(error))?;
        self.owners.remove(&key.get());
        Ok(())
    }
}
