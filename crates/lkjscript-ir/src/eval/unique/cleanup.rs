use super::{map_store_error, owner_word, EvalUniqueRuntime};
use crate::eval::{EvalValue, Flow};
use lkjscript_core::UniqueKeyWord;

impl EvalUniqueRuntime {
    pub(crate) fn drop_owner(&mut self, value: EvalValue) -> Result<(), Flow> {
        let word = owner_word(&value)?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Flow::Trap(
                "evaluator owner drop precedes end-borrow".into(),
            ));
        }
        let key = self
            .store
            .import_byte_vector_key(word)
            .map_err(map_store_error)?;
        self.store.free_byte_vector(key).map_err(map_store_error)?;
        if !self.owners.remove(&word.get()) {
            return Err(Flow::Trap("duplicate evaluator owner drop".into()));
        }
        Ok(())
    }

    pub(crate) fn export_owner(&mut self, value: EvalValue) -> Result<Vec<u8>, Flow> {
        let word = owner_word(&value)?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Flow::Trap("returned owner has a live loan".into()));
        }
        let key = self
            .store
            .import_byte_vector_key(word)
            .map_err(map_store_error)?;
        let bytes = self.store.take_byte_vector(key).map_err(map_store_error)?;
        if !self.owners.remove(&word.get()) {
            return Err(Flow::Trap("returned owner is not live".into()));
        }
        Ok(bytes)
    }

    pub(crate) fn cleanup(&mut self) -> Result<(), Flow> {
        self.loans.clear();
        let owners: Vec<u64> = self.owners.iter().copied().collect();
        for owner in owners {
            let word = UniqueKeyWord::new(owner).map_err(|error| Flow::Trap(error.to_string()))?;
            let key = self
                .store
                .import_byte_vector_key(word)
                .map_err(map_store_error)?;
            self.store.free_byte_vector(key).map_err(map_store_error)?;
            self.owners.remove(&owner);
        }
        self.verify_empty()
    }

    pub(crate) fn verify_empty(&self) -> Result<(), Flow> {
        if !self.loans.is_empty() || !self.owners.is_empty() {
            return Err(Flow::Trap(
                "evaluator unique runtime retained owners or loans".into(),
            ));
        }
        self.store
            .assert_no_leaks()
            .map_err(|error| Flow::Trap(error.to_string()))
    }
}
