use super::*;

impl JitUniqueRuntime {
    pub(crate) fn drop_owner(&mut self, owner: NativeUnique) -> Result<(), NativeServiceError> {
        let kind = owner.unique_type();
        let word = self.validate_owner(owner, kind)?;
        if self.active_loans_for(word).next().is_some() {
            return Err(self.reject());
        }
        let result = match kind {
            lkjscript_native::UniqueType::ByteVector => self
                .store
                .import_byte_vector_key(word)
                .and_then(|key| self.store.free_byte_vector(key)),
            lkjscript_native::UniqueType::Bytes => self
                .store
                .import_bytes_key(word)
                .and_then(|key| self.store.free_bytes(key)),
        };
        result.map_err(|error| self.store_error(error))?;
        self.remove_owner(word.get())?;
        self.stats.drops = self.stats.drops.saturating_add(1);
        Ok(())
    }

    pub(crate) fn export_owner(
        &mut self,
        owner: NativeUnique,
    ) -> Result<Vec<u8>, NativeServiceError> {
        let kind = owner.unique_type();
        let word = self.validate_owner(owner, kind)?;
        if self.active_loans_for(word).next().is_some() {
            return Err(self.reject());
        }
        let bytes = match kind {
            lkjscript_native::UniqueType::ByteVector => self
                .store
                .import_byte_vector_key(word)
                .and_then(|key| self.store.take_byte_vector(key)),
            lkjscript_native::UniqueType::Bytes => self
                .store
                .import_bytes_key(word)
                .and_then(|key| self.store.take_bytes(key)),
        }
        .map_err(|error| self.store_error(error))?;
        self.remove_owner(word.get())?;
        self.stats.transfers = self.stats.transfers.saturating_add(1);
        Ok(bytes)
    }

    pub(crate) const fn last_resource(&self) -> Option<ResourceLimitKind> {
        self.last_resource
    }

    pub(crate) fn finish(&mut self) -> NativeUniqueStats {
        for slot in &mut self.loans {
            if slot.loan.take().is_some() {
                self.stats.cleanup_attempts = self.stats.cleanup_attempts.saturating_add(1);
            }
        }
        while let Some(owner) = self.owners.pop() {
            self.stats.cleanup_attempts = self.stats.cleanup_attempts.saturating_add(1);
            let result = UniqueKeyWord::new(owner)
                .map_err(|_| UniqueStoreError::StaleKey)
                .and_then(|word| {
                    self.store
                        .import_byte_vector_key(word)
                        .and_then(|key| self.store.free_byte_vector(key))
                        .or_else(|_| {
                            self.store
                                .import_bytes_key(word)
                                .and_then(|key| self.store.free_bytes(key))
                        })
                });
            if result.is_ok() {
                self.stats.cleanup_releases = self.stats.cleanup_releases.saturating_add(1);
            } else {
                self.stats.teardown_failures = self.stats.teardown_failures.saturating_add(1);
            }
        }
        let store_stats = self.store.stats();
        self.stats.live_owners = u64::from(store_stats.live_objects);
        self.stats.live_loans = self.loans.iter().filter(|slot| slot.loan.is_some()).count() as u64;
        self.stats.release_backlog = self.stats.live_owners;
        if self.store.assert_no_leaks().is_err()
            || self.stats.live_owners != 0
            || self.stats.live_loans != 0
        {
            self.stats.teardown_failures = self.stats.teardown_failures.saturating_add(1);
        }
        self.stats
    }

    fn remove_owner(&mut self, word: u64) -> Result<(), NativeServiceError> {
        let Some(index) = self.owners.iter().position(|owner| *owner == word) else {
            return Err(self.reject());
        };
        self.owners.swap_remove(index);
        Ok(())
    }
}
