use super::{
    map_store_error, Error, Loan, Result, UniqueKeyWord, UniqueLayout, UniqueRuntime, Value,
};

impl UniqueRuntime {
    pub(crate) fn allocate_bytes(&mut self, bytes: Vec<u8>) -> Result<Value> {
        self.preflight_allocation(bytes.capacity())?;
        let key = self.store.allocate_bytes(bytes).map_err(map_store_error)?;
        self.publish_bytes(key.packed_word().get())
    }

    pub(crate) fn copy_bytes(&mut self, value: Value) -> Result<Vec<u8>> {
        let word = self.bytes_word(value)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        Ok(self.store.bytes(key).map_err(map_store_error)?.to_vec())
    }

    pub(crate) fn copy_owner_bytes(&mut self, value: Value) -> Result<Vec<u8>> {
        let owner = self.validate_any_owner(value)?;
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        if value.as_bytes_key().is_some() {
            let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
            Ok(self.store.bytes(key).map_err(map_store_error)?.to_vec())
        } else if value.as_byte_vector_key().is_some() {
            let key = self
                .store
                .import_byte_vector_key(word)
                .map_err(map_store_error)?;
            Ok(self
                .store
                .byte_vector(key)
                .map_err(map_store_error)?
                .to_vec())
        } else {
            Err(Error::msg("expected exact unique byte owner"))
        }
    }

    pub(crate) fn bytes_length(&mut self, value: Value) -> Result<usize> {
        let word = self.bytes_word(value)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        Ok(self.store.bytes(key).map_err(map_store_error)?.len())
    }

    pub(crate) fn bytes_at(&mut self, value: Value, index: usize) -> Result<u8> {
        let word = self.bytes_word(value)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        self.store
            .bytes(key)
            .map_err(map_store_error)?
            .get(index)
            .copied()
            .ok_or_else(|| Error::msg(format!("bytes-byte-at index {index} out of range")))
    }

    pub(crate) fn clone_bytes(&mut self, value: Value) -> Result<Value> {
        let word = self.bytes_word(value)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        let bytes = self.store.bytes(key).map_err(map_store_error)?.len();
        self.preflight_allocation(bytes)?;
        let clone = self.store.clone_bytes(key).map_err(map_store_error)?;
        self.publish_bytes(clone.packed_word().get())
    }

    pub(crate) fn copy_bytes_range(
        &mut self,
        value: Value,
        start: usize,
        len: usize,
    ) -> Result<Value> {
        let word = self.bytes_word(value)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        self.preflight_allocation(len)?;
        let clone = self
            .store
            .clone_bytes_range(key, start, len)
            .map_err(map_store_error)?;
        self.publish_bytes(clone.packed_word().get())
    }

    pub(crate) fn clone_static(&mut self, bytes: &[u8]) -> Result<Value> {
        self.preflight_allocation(bytes.len())?;
        let key = self
            .store
            .clone_static_bytes(bytes)
            .map_err(map_store_error)?;
        self.publish_bytes(key.packed_word().get())
    }

    pub(crate) fn copy_static_range(
        &mut self,
        bytes: &[u8],
        start: usize,
        len: usize,
    ) -> Result<Value> {
        self.preflight_allocation(len)?;
        let key = self
            .store
            .clone_static_bytes_range(bytes, start, len)
            .map_err(map_store_error)?;
        self.publish_bytes(key.packed_word().get())
    }

    pub(crate) fn freeze(&mut self, value: Value) -> Result<Value> {
        let owner = self.ensure_unloaned(value)?;
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        let key = self
            .store
            .import_byte_vector_key(word)
            .map_err(map_store_error)?;
        self.store
            .freeze_byte_vector(key)
            .map_err(map_store_error)?;
        self.owners.insert(owner, UniqueLayout::Bytes);
        Ok(Value::from_bytes_key(owner))
    }

    pub(crate) fn thaw_dynamic(&mut self, value: Value) -> Result<Value> {
        let owner = value
            .as_bytes_key()
            .ok_or_else(|| Error::msg("thaw expects dynamic bytes owner"))?;
        if self.owners.get(&owner) != Some(&UniqueLayout::Bytes) {
            return Err(Error::msg("stale or forged bytes owner"));
        }
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        if self.loans.values().any(|loan| loan.owner == word) {
            return Err(Error::msg("thaw bytes owner has a live loan"));
        }
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        self.store
            .thaw_dynamic_bytes(key)
            .map_err(map_store_error)?;
        self.owners.insert(owner, UniqueLayout::ByteVector);
        Ok(Value::from_byte_vector_key(owner))
    }

    pub(crate) fn thaw_static(&mut self, bytes: &[u8]) -> Result<Value> {
        self.preflight_allocation(bytes.len())?;
        let key = self
            .store
            .thaw_bytes_slice(bytes)
            .map_err(map_store_error)?;
        let owner = key.packed_word().get();
        if self
            .owners
            .insert(owner, UniqueLayout::ByteVector)
            .is_some()
        {
            return Err(Error::msg("duplicate static thaw owner"));
        }
        Ok(Value::from_byte_vector_key(owner))
    }

    pub(crate) fn borrow_bytes(&mut self, value: Value) -> Result<Value> {
        let owner = value
            .as_bytes_key()
            .ok_or_else(|| Error::msg("bytes borrow expects dynamic owner"))?;
        if self.owners.get(&owner) != Some(&UniqueLayout::Bytes) {
            return Err(Error::msg("stale or forged bytes owner"));
        }
        let word = UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        let len = self.store.bytes(key).map_err(map_store_error)?.len();
        let token = self.next_loan;
        self.next_loan = self
            .next_loan
            .checked_add(1)
            .ok_or_else(|| Error::msg("bytes loan identity overflow"))?;
        self.loans.insert(
            token,
            Loan {
                owner: word,
                mutable: false,
                start: 0,
                len,
            },
        );
        Ok(Value::from_bytes_borrow(token))
    }

    fn bytes_word(&self, value: Value) -> Result<UniqueKeyWord> {
        let owner = if let Some(owner) = value.as_bytes_key() {
            owner
        } else if let Some(token) = value.as_bytes_borrow() {
            self.loans
                .get(&token)
                .filter(|loan| !loan.mutable)
                .map(|loan| loan.owner.get())
                .ok_or_else(|| Error::msg("stale bytes borrow"))?
        } else {
            return Err(Error::msg("expected exact bytes value"));
        };
        if self.owners.get(&owner) != Some(&UniqueLayout::Bytes) {
            return Err(Error::msg("stale, forged, or wrong-layout bytes owner"));
        }
        UniqueKeyWord::new(owner).map_err(|error| Error::msg(error.to_string()))
    }

    fn publish_bytes(&mut self, owner: u64) -> Result<Value> {
        if self.owners.insert(owner, UniqueLayout::Bytes).is_some() {
            return Err(Error::msg("duplicate bytes owner"));
        }
        Ok(Value::from_bytes_key(owner))
    }
}
