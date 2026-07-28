use super::{Error, Result, UniqueKeyWord, UniqueLayout, UniqueRuntime, Value};
use lkjscript_core::{OwnedValue, ResourceLimitKind, UniqueStoreError};

pub(super) fn map_store_error(error: UniqueStoreError) -> Error {
    match error {
        UniqueStoreError::AllocationLimit | UniqueStoreError::ObjectLimit => Error::resource(
            ResourceLimitKind::Allocations,
            "unique byte-vector allocation limit exceeded",
        ),
        UniqueStoreError::ByteLimit
        | UniqueStoreError::SlotLimit
        | UniqueStoreError::StorageCapacity => Error::resource(
            ResourceLimitKind::HeapBytes,
            "unique byte-vector heap limit exceeded",
        ),
        _ => Error::msg(error.to_string()),
    }
}

impl UniqueRuntime {
    pub(crate) fn drop_owner(&mut self, value: Value) -> Result<()> {
        let (owner, layout) = exact_owner(value)?;
        self.require(owner, layout)?;
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Error::msg("VM unique Drop precedes EndBorrow"));
        }
        self.free(word, layout)?;
        if self.owners.remove(&owner) != Some(layout) {
            return Err(Error::msg("VM duplicate or wrong-layout unique Drop"));
        }
        Ok(())
    }

    pub(crate) fn export_owner(&mut self, value: Value) -> Result<OwnedValue> {
        let (owner, layout) = exact_owner(value)?;
        self.require(owner, layout)?;
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Error::msg("returned VM unique owner has a live loan"));
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
            UniqueLayout::Path => return Err(Error::msg("path is not a unique VM return")),
        };
        self.owners.remove(&owner);
        match layout {
            UniqueLayout::ByteVector => OwnedValue::from_unique_byte_vector(bytes),
            UniqueLayout::Bytes => OwnedValue::from_unique_bytes(bytes),
            UniqueLayout::Path => unreachable!("path rejected above"),
        }
    }

    pub(crate) fn cleanup(&mut self) -> Result<()> {
        self.loans.clear();
        let owners: Vec<_> = self
            .owners
            .iter()
            .map(|(word, layout)| (*word, *layout))
            .collect();
        for (owner, layout) in owners {
            let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
            self.free(word, layout)?;
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

    fn require(&self, owner: u64, layout: UniqueLayout) -> Result<()> {
        if self.owners.get(&owner) == Some(&layout) {
            Ok(())
        } else {
            Err(Error::msg("VM stale, forged, or wrong-layout owner"))
        }
    }

    fn free(&mut self, word: UniqueKeyWord, layout: UniqueLayout) -> Result<()> {
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
            UniqueLayout::Path => Err(Error::msg("path is not a registered VM owner")),
        }
    }
}

fn exact_owner(value: Value) -> Result<(u64, UniqueLayout)> {
    if let Some(word) = value.as_bytes_key() {
        return Ok((word, UniqueLayout::Bytes));
    }
    if let Some(word) = value.as_byte_vector_key() {
        return Ok((word, UniqueLayout::ByteVector));
    }
    Err(Error::msg("expected exact VM unique owner"))
}
