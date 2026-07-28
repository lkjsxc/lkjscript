use super::{map_store_error, EvalUniqueRuntime};
use crate::eval::{EvalValue, Flow};
use lkjscript_core::{UniqueKeyWord, UniqueLayout};

impl EvalUniqueRuntime {
    pub(crate) fn bytes_length(&mut self, value: &EvalValue) -> Result<usize, Flow> {
        let word = self.bytes_word(value)?;
        self.require_owner(word, UniqueLayout::Bytes)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        Ok(self.store.bytes(key).map_err(map_store_error)?.len())
    }

    pub(crate) fn bytes_at(&mut self, value: &EvalValue, index: usize) -> Result<u8, Flow> {
        let word = self.bytes_word(value)?;
        self.require_owner(word, UniqueLayout::Bytes)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        self.store
            .bytes(key)
            .map_err(map_store_error)?
            .get(index)
            .copied()
            .ok_or_else(|| Flow::Trap("bytes-byte-at index out of range".into()))
    }

    pub(crate) fn clone_bytes(&mut self, value: &EvalValue) -> Result<EvalValue, Flow> {
        let word = self.bytes_word(value)?;
        self.require_owner(word, UniqueLayout::Bytes)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        let clone = self.store.clone_bytes(key).map_err(map_store_error)?;
        self.publish_bytes(clone.packed_word())
    }

    pub(crate) fn copy_bytes_range(
        &mut self,
        value: &EvalValue,
        start: usize,
        len: usize,
    ) -> Result<EvalValue, Flow> {
        let word = self.bytes_word(value)?;
        self.require_owner(word, UniqueLayout::Bytes)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        let clone = self
            .store
            .clone_bytes_range(key, start, len)
            .map_err(map_store_error)?;
        self.publish_bytes(clone.packed_word())
    }

    pub(crate) fn clone_static(&mut self, bytes: &[u8]) -> Result<EvalValue, Flow> {
        let key = self
            .store
            .clone_static_bytes(bytes)
            .map_err(map_store_error)?;
        self.publish_bytes(key.packed_word())
    }

    pub(crate) fn copy_static_range(
        &mut self,
        bytes: &[u8],
        start: usize,
        len: usize,
    ) -> Result<EvalValue, Flow> {
        let key = self
            .store
            .clone_static_bytes_range(bytes, start, len)
            .map_err(map_store_error)?;
        self.publish_bytes(key.packed_word())
    }

    pub(crate) fn freeze(&mut self, value: &EvalValue) -> Result<EvalValue, Flow> {
        let EvalValue::ByteVector(word) = value else {
            return Err(Flow::Trap(
                "freeze-byte-vector expects byte-vector owner".into(),
            ));
        };
        self.require_owner(*word, UniqueLayout::ByteVector)?;
        if self.loans.values().any(|loan| loan.owner == *word) {
            return Err(Flow::Trap(
                "freeze-byte-vector owner has a live loan".into(),
            ));
        }
        let key = self
            .store
            .import_byte_vector_key(*word)
            .map_err(map_store_error)?;
        self.store
            .freeze_byte_vector(key)
            .map_err(map_store_error)?;
        self.owners.insert(word.get(), UniqueLayout::Bytes);
        Ok(EvalValue::Bytes(*word))
    }

    pub(crate) fn thaw_dynamic(&mut self, value: &EvalValue) -> Result<EvalValue, Flow> {
        let word = self.bytes_word(value)?;
        self.require_owner(word, UniqueLayout::Bytes)?;
        let key = self.store.import_bytes_key(word).map_err(map_store_error)?;
        self.store
            .thaw_dynamic_bytes(key)
            .map_err(map_store_error)?;
        self.owners.insert(word.get(), UniqueLayout::ByteVector);
        Ok(EvalValue::ByteVector(word))
    }

    pub(crate) fn thaw_static(&mut self, bytes: &[u8]) -> Result<EvalValue, Flow> {
        let key = self
            .store
            .thaw_bytes_slice(bytes)
            .map_err(map_store_error)?;
        let word = key.packed_word();
        if self
            .owners
            .insert(word.get(), UniqueLayout::ByteVector)
            .is_some()
        {
            return Err(Flow::Trap("duplicate evaluator thaw owner".into()));
        }
        Ok(EvalValue::ByteVector(word))
    }

    fn bytes_word(&self, value: &EvalValue) -> Result<UniqueKeyWord, Flow> {
        match value {
            EvalValue::Bytes(word) => Ok(*word),
            EvalValue::BytesBorrow(token) => self
                .loans
                .get(token)
                .filter(|loan| !loan.mutable)
                .map(|loan| loan.owner)
                .ok_or_else(|| Flow::Trap("stale immutable-bytes borrow".into())),
            _ => Err(Flow::Trap("expected immutable bytes".into())),
        }
    }

    fn publish_bytes(&mut self, word: UniqueKeyWord) -> Result<EvalValue, Flow> {
        if self
            .owners
            .insert(word.get(), UniqueLayout::Bytes)
            .is_some()
        {
            return Err(Flow::Trap("duplicate evaluator bytes owner".into()));
        }
        Ok(EvalValue::Bytes(word))
    }

    fn require_owner(&self, word: UniqueKeyWord, layout: UniqueLayout) -> Result<(), Flow> {
        if self.owners.get(&word.get()) == Some(&layout) {
            Ok(())
        } else {
            Err(Flow::Trap(
                "stale, forged, or wrong-layout evaluator owner".into(),
            ))
        }
    }
}
