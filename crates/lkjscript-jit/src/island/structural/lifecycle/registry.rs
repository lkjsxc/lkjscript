use super::*;

impl JitStructuralRuntime {
    pub(in crate::island::structural) fn require_owner(
        &self,
        owner: NativeStructuralOwner,
        storage: Option<StructuralStorageRoute>,
    ) -> Result<NativeOwnerRecord, NativeServiceError> {
        let key = owner_key(owner)?;
        let record = self
            .owners
            .get(&key.get())
            .copied()
            .ok_or(NativeServiceError::Trap)?;
        if record.value_type != owner.structural_type()
            || storage.is_some_and(|expected| expected != record.storage)
        {
            return Err(NativeServiceError::Trap);
        }
        Ok(record)
    }

    pub(in crate::island::structural) fn register_owner(
        &mut self,
        key: u64,
        value_type: StructuralTypeIdentity,
        storage: StructuralStorageRoute,
    ) -> Result<(), NativeServiceError> {
        if self.owners.contains_key(&key) {
            return Err(NativeServiceError::HostFailure);
        }
        self.owners.insert(
            key,
            NativeOwnerRecord {
                value_type,
                storage,
            },
        );
        Ok(())
    }

    pub(in crate::island::structural) fn register_runtime_owner(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
        value_type: StructuralTypeIdentity,
        storage: StructuralStorageRoute,
    ) -> Result<(), NativeServiceError> {
        if let Err(registration) = self.register_owner(key.get(), value_type, storage) {
            if let Err(cleanup) = self.runtime.dispose_owner(key, expected) {
                return Err(self.map_error(cleanup));
            }
            return Err(registration);
        }
        Ok(())
    }

    pub(in crate::island::structural) fn replace_owner(
        &mut self,
        old: u64,
        new: u64,
        value_type: StructuralTypeIdentity,
        storage: StructuralStorageRoute,
    ) -> Result<(), NativeServiceError> {
        let record = self
            .owners
            .get(&old)
            .copied()
            .ok_or(NativeServiceError::HostFailure)?;
        if record.value_type != value_type || (old != new && self.owners.contains_key(&new)) {
            return Err(NativeServiceError::HostFailure);
        }
        self.owners.remove(&old);
        self.owners.insert(
            new,
            NativeOwnerRecord {
                value_type,
                storage,
            },
        );
        Ok(())
    }

    pub(in crate::island::structural) fn replace_runtime_owner(
        &mut self,
        old: StructuralValueKey,
        new: StructuralValueKey,
        expected: StructuralType,
        value_type: StructuralTypeIdentity,
        storage: StructuralStorageRoute,
    ) -> Result<(), NativeServiceError> {
        if let Err(registration) = self.replace_owner(old.get(), new.get(), value_type, storage) {
            if let Err(cleanup) = self.runtime.dispose_owner(new, expected) {
                return Err(self.map_error(cleanup));
            }
            return Err(registration);
        }
        Ok(())
    }
}
