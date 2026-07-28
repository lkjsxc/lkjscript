use std::collections::BTreeMap;

use lkjscript_core::{
    Error, ExecutionConfig, Result, UniqueKeyWord, UniqueLayout, UniqueStore, UniqueStoreId,
    UniqueStoreLimits, Value, MAX_BUFFER_BYTES,
};

mod access;
mod bytes;
mod cleanup;

use cleanup::map_store_error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimePlace {
    Inactive,
    Active {
        owner: Option<u64>,
        transferred: Option<u64>,
    },
}

#[derive(Clone, Copy, Debug)]
struct Loan {
    owner: UniqueKeyWord,
    mutable: bool,
    start: usize,
    len: usize,
}

pub(crate) struct UniqueRuntime {
    store: UniqueStore,
    owners: BTreeMap<u64, UniqueLayout>,
    loans: BTreeMap<u64, Loan>,
    next_loan: u64,
}

impl UniqueRuntime {
    pub(crate) fn new(config: &ExecutionConfig) -> Self {
        let objects = u32::try_from(config.max_allocations).unwrap_or(u32::MAX);
        let bytes = u64::try_from(config.max_heap_bytes).unwrap_or(u64::MAX);
        let Some(id) = UniqueStoreId::new(1) else {
            unreachable!("nonzero VM unique-store identity")
        };
        let Ok(limits) =
            UniqueStoreLimits::new(objects, bytes, objects, config.max_allocations, u32::MAX)
        else {
            unreachable!("VM unique-store limits are constructed consistently")
        };
        Self {
            store: UniqueStore::new(id, limits),
            owners: BTreeMap::new(),
            loans: BTreeMap::new(),
            next_loan: 1_u64 << 63,
        }
    }

    pub(crate) fn allocate(&mut self, size: i64) -> Result<Value> {
        let size = usize::try_from(size)
            .ok()
            .filter(|size| *size <= MAX_BUFFER_BYTES)
            .ok_or_else(|| Error::msg("new-byte-vector size out of range"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| Error::msg("byte-vector backing capacity unavailable"))?;
        bytes.resize(size, 0);
        let key = self
            .store
            .allocate_byte_vector(bytes)
            .map_err(map_store_error)?;
        let word = key.packed_word().get();
        if self.owners.insert(word, UniqueLayout::ByteVector).is_some() {
            return Err(Error::msg("VM duplicate byte-vector owner"));
        }
        Ok(Value::from_byte_vector_key(word))
    }

    pub(crate) fn validate_owner(&mut self, value: Value) -> Result<u64> {
        let word = value
            .as_byte_vector_key()
            .ok_or_else(|| Error::msg("expected exact byte-vector key"))?;
        if self.owners.get(&word) != Some(&UniqueLayout::ByteVector) {
            return Err(Error::msg("VM stale or moved byte-vector owner"));
        }
        let word = UniqueKeyWord::new(word).map_err(|error| Error::msg(error.to_string()))?;
        self.store
            .import_byte_vector_key(word)
            .map_err(map_store_error)?;
        Ok(word.get())
    }

    pub(crate) fn validate_any_owner(&mut self, value: Value) -> Result<u64> {
        let (word, layout) = if let Some(word) = value.as_bytes_key() {
            (word, UniqueLayout::Bytes)
        } else if let Some(word) = value.as_byte_vector_key() {
            (word, UniqueLayout::ByteVector)
        } else {
            return Err(Error::msg("expected exact dynamic unique owner"));
        };
        if self.owners.get(&word) != Some(&layout) {
            return Err(Error::msg("stale, forged, or wrong-layout unique owner"));
        }
        Ok(word)
    }

    pub(crate) fn ensure_any_unloaned(&mut self, value: Value) -> Result<u64> {
        let owner = self.validate_any_owner(value)?;
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Error::msg("VM unique owner has a live loan"));
        }
        Ok(owner)
    }

    pub(crate) fn ensure_unloaned(&mut self, value: Value) -> Result<u64> {
        let owner = self.validate_owner(value)?;
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Error::msg("VM byte-vector owner has a live loan"));
        }
        Ok(owner)
    }

    pub(crate) fn borrow(&mut self, owner: Value, mutable: bool) -> Result<Value> {
        let owner = self.validate_owner(owner)?;
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        if self
            .loans
            .values()
            .any(|loan| loan.owner == word && (mutable || loan.mutable))
        {
            return Err(Error::msg("conflicting VM byte-vector loan"));
        }
        let key = self
            .store
            .import_byte_vector_key(word)
            .map_err(map_store_error)?;
        let len = self.store.byte_vector(key).map_err(map_store_error)?.len();
        let token = self.next_loan;
        self.next_loan = self
            .next_loan
            .checked_add(1)
            .ok_or_else(|| Error::msg("VM byte-view identity overflow"))?;
        self.loans.insert(
            token,
            Loan {
                owner: word,
                mutable,
                start: 0,
                len,
            },
        );
        Ok(Value::from_byte_slice(token, mutable))
    }

    pub(crate) fn validate_any_view(&self, value: Value) -> Result<u64> {
        let token = value
            .as_bytes_borrow()
            .or_else(|| value.as_byte_slice().map(|(token, _)| token))
            .ok_or_else(|| Error::msg("expected exact byte view"))?;
        self.loans
            .contains_key(&token)
            .then_some(token)
            .ok_or_else(|| Error::msg("VM stale byte view"))
    }

    pub(crate) fn validate_view(&self, value: Value, mutable: bool) -> Result<u64> {
        let (token, value_mutable) = value
            .as_byte_slice()
            .ok_or_else(|| Error::msg("expected exact byte view"))?;
        self.loans
            .get(&token)
            .filter(|loan| loan.mutable == mutable && value_mutable == mutable)
            .map(|_| token)
            .ok_or_else(|| Error::msg("VM stale or wrong byte-view type"))
    }

    pub(crate) fn end_borrow(&mut self, value: Value) -> Result<()> {
        let token = value
            .as_bytes_borrow()
            .or_else(|| value.as_byte_slice().map(|(token, _)| token))
            .ok_or_else(|| Error::msg("expected exact byte view"))?;
        self.loans
            .remove(&token)
            .map(|_| ())
            .ok_or_else(|| Error::msg("VM stale byte view at EndBorrow"))
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
