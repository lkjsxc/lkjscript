use super::{map_store_error, Error, Result, UniqueKeyWord, UniqueRuntime, Value};
use lkjscript_core::OwnedValue;

impl UniqueRuntime {
    pub(crate) fn drop_owner(&mut self, value: Value) -> Result<()> {
        let owner = self.validate_owner(value)?;
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Error::msg("VM byte-vector Drop precedes EndBorrow"));
        }
        let key = self
            .store
            .import_byte_vector_key(word)
            .map_err(map_store_error)?;
        self.store.free_byte_vector(key).map_err(map_store_error)?;
        if !self.owners.remove(&owner) {
            return Err(Error::msg("VM duplicate byte-vector Drop"));
        }
        Ok(())
    }

    pub(crate) fn export_owner(&mut self, value: Value) -> Result<OwnedValue> {
        let owner = self.validate_owner(value)?;
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Error::msg("returned VM byte-vector has a live loan"));
        }
        let key = self
            .store
            .import_byte_vector_key(word)
            .map_err(map_store_error)?;
        let bytes = self.store.take_byte_vector(key).map_err(map_store_error)?;
        self.owners.remove(&owner);
        OwnedValue::from_unique_byte_vector(bytes)
    }

    pub(crate) fn cleanup(&mut self) -> Result<()> {
        self.loans.clear();
        let owners: Vec<u64> = self.owners.iter().copied().collect();
        for owner in owners {
            let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
            let key = self
                .store
                .import_byte_vector_key(word)
                .map_err(map_store_error)?;
            self.store.free_byte_vector(key).map_err(map_store_error)?;
            self.owners.remove(&owner);
        }
        self.verify_empty()
    }

    pub(crate) fn verify_empty(&self) -> Result<()> {
        if !self.owners.is_empty() || !self.loans.is_empty() {
            return Err(Error::msg("VM unique runtime retained owners or loans"));
        }
        self.store
            .assert_no_leaks()
            .map_err(|error| Error::msg(error.to_string()))
    }
}
