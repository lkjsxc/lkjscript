use super::EvalUniqueRuntime;
use crate::eval::{EvalValue, Flow};
use lkjscript_core::{UniqueKeyWord, UniqueLayout, UniqueStoreError};

pub(super) fn map_store_error(error: UniqueStoreError) -> Flow {
    match error {
        UniqueStoreError::AllocationLimit | UniqueStoreError::ObjectLimit => {
            Flow::Resource("allocations".into())
        }
        UniqueStoreError::ByteLimit
        | UniqueStoreError::SlotLimit
        | UniqueStoreError::StorageCapacity => Flow::Resource("heap bytes".into()),
        _ => Flow::Trap(error.to_string()),
    }
}

impl EvalUniqueRuntime {
    pub(crate) fn drop_owner(&mut self, value: EvalValue) -> Result<(), Flow> {
        let (word, layout) = owner(&value)?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Flow::Trap(
                "evaluator owner drop precedes end-borrow".into(),
            ));
        }
        self.release(word, layout)?;
        if self.owners.remove(&word.get()) != Some(layout) {
            return Err(Flow::Trap(
                "duplicate or wrong-layout evaluator owner drop".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn export_owner(&mut self, value: EvalValue) -> Result<Vec<u8>, Flow> {
        let (word, layout) = owner(&value)?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Flow::Trap("returned owner has a live loan".into()));
        }
        if self.owners.get(&word.get()) != Some(&layout) {
            return Err(Flow::Trap("returned owner is stale or forged".into()));
        }
        let bytes = match layout {
            UniqueLayout::ByteVector => {
                let key = self
                    .store
                    .import_byte_vector_key(word)
                    .map_err(map_store_error)?;
                self.store.take_byte_vector(key).map_err(map_store_error)?
            }
            UniqueLayout::Bytes => {
                let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
                self.store.take_bytes(key).map_err(map_store_error)?
            }
            UniqueLayout::Path => {
                let key = self.store.import_path_key(word).map_err(map_store_error)?;
                self.store
                    .take_path(key)
                    .map_err(map_store_error)?
                    .into_vec()
            }
        };
        if self.owners.remove(&word.get()) != Some(layout) {
            return Err(Flow::Trap("returned owner is not live".into()));
        }
        Ok(bytes)
    }

    pub(crate) fn cleanup(&mut self) -> Result<(), Flow> {
        self.loans.clear();
        let owners: Vec<_> = self
            .owners
            .iter()
            .map(|(word, layout)| (*word, *layout))
            .collect();
        for (owner, layout) in owners {
            let word = UniqueKeyWord::new(owner).map_err(|error| Flow::Trap(error.to_string()))?;
            self.release(word, layout)?;
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

    fn release(&mut self, word: UniqueKeyWord, layout: UniqueLayout) -> Result<(), Flow> {
        match layout {
            UniqueLayout::ByteVector => {
                let key = self
                    .store
                    .import_byte_vector_key(word)
                    .map_err(map_store_error)?;
                self.store.free_byte_vector(key).map_err(map_store_error)
            }
            UniqueLayout::Bytes => {
                let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
                self.store.free_bytes(key).map_err(map_store_error)
            }
            UniqueLayout::Path => {
                let key = self.store.import_path_key(word).map_err(map_store_error)?;
                self.store.free_path(key).map_err(map_store_error)
            }
        }
    }
}

fn owner(value: &EvalValue) -> Result<(UniqueKeyWord, UniqueLayout), Flow> {
    match value {
        EvalValue::ByteVector(word) => Ok((*word, UniqueLayout::ByteVector)),
        EvalValue::Bytes(word) => Ok((*word, UniqueLayout::Bytes)),
        EvalValue::Path(word) => Ok((*word, UniqueLayout::Path)),
        _ => Err(Flow::Trap("expected exact unique owner".into())),
    }
}
