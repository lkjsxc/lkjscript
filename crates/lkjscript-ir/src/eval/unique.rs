use std::collections::BTreeMap;

use lkjscript_core::{UniqueKeyWord, UniqueLayout, UniqueStore, UniqueStoreId, UniqueStoreLimits};

use super::{EvalConfig, EvalValue, Flow};

mod bytes;
mod cleanup;
mod path;
mod word;

use cleanup::map_store_error;

#[derive(Clone, Copy, Debug)]
struct Loan {
    owner: UniqueKeyWord,
    mutable: bool,
    start: usize,
    len: usize,
}

pub(crate) struct EvalUniqueRuntime {
    store: UniqueStore,
    owners: BTreeMap<u64, UniqueLayout>,
    loans: BTreeMap<u64, Loan>,
    next_loan: u64,
}

impl EvalUniqueRuntime {
    pub(super) fn new(config: &EvalConfig) -> Option<Self> {
        let objects = u32::try_from(config.max_allocations).unwrap_or(u32::MAX);
        let bytes = u64::try_from(config.max_heap_bytes).unwrap_or(u64::MAX);
        let id = UniqueStoreId::new(1)?;
        let limits =
            UniqueStoreLimits::new(objects, bytes, objects, config.max_allocations, u32::MAX)
                .ok()?;
        Some(Self {
            store: UniqueStore::new(id, limits),
            owners: BTreeMap::new(),
            loans: BTreeMap::new(),
            next_loan: 1,
        })
    }

    pub(super) fn allocate(&mut self, size: usize) -> Result<EvalValue, Flow> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| Flow::Resource("heap bytes".into()))?;
        bytes.resize(size, 0);
        let key = self
            .store
            .allocate_byte_vector(bytes)
            .map_err(map_store_error)?;
        let word = key.packed_word();
        if self
            .owners
            .insert(word.get(), UniqueLayout::ByteVector)
            .is_some()
        {
            return Err(Flow::Trap("duplicate evaluator byte-vector owner".into()));
        }
        Ok(EvalValue::ByteVector(word))
    }

    pub(super) fn borrow(&mut self, owner: &EvalValue, mutable: bool) -> Result<EvalValue, Flow> {
        let (word, len, bytes_borrow) = match owner {
            EvalValue::ByteVector(word) => {
                let key = self
                    .store
                    .import_byte_vector_key(*word)
                    .map_err(map_store_error)?;
                let len = self.store.byte_vector(key).map_err(map_store_error)?.len();
                (*word, len, false)
            }
            EvalValue::Bytes(word) if !mutable => {
                let key = self
                    .store
                    .import_bytes_key(*word)
                    .map_err(map_store_error)?;
                let len = self.store.bytes(key).map_err(map_store_error)?.len();
                (*word, len, true)
            }
            _ => return Err(Flow::Trap("expected exact borrowable unique owner".into())),
        };
        if self
            .loans
            .values()
            .any(|loan| loan.owner == word && (mutable || loan.mutable))
        {
            return Err(Flow::Trap("conflicting evaluator byte-vector loan".into()));
        }
        let token = self.next_loan;
        self.next_loan = self
            .next_loan
            .checked_add(1)
            .ok_or_else(|| Flow::Trap("evaluator loan identity overflow".into()))?;
        self.loans.insert(
            token,
            Loan {
                owner: word,
                mutable,
                start: 0,
                len,
            },
        );
        Ok(if bytes_borrow {
            EvalValue::BytesBorrow(token)
        } else if mutable {
            EvalValue::ByteSliceMut(token)
        } else {
            EvalValue::ByteSlice(token)
        })
    }

    pub(super) fn end_borrow(&mut self, value: EvalValue) -> Result<(), Flow> {
        let token = view_token(&value)?;
        self.loans
            .remove(&token)
            .ok_or_else(|| Flow::Trap("stale evaluator byte view".into()))?;
        Ok(())
    }

    pub(super) fn len(&mut self, value: &EvalValue) -> Result<i64, Flow> {
        let loan = self.shared_loan(value)?;
        i64::try_from(loan.len).map_err(|_| Flow::Trap("byte-slice length out of range".into()))
    }

    pub(super) fn byte_at(&mut self, value: &EvalValue, index: usize) -> Result<i64, Flow> {
        let loan = self.shared_loan(value)?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(map_store_error)?;
        let bytes = self
            .store
            .byte_vector_range(key, loan.start, loan.len)
            .map_err(map_store_error)?;
        bytes
            .get(index)
            .copied()
            .map(i64::from)
            .ok_or_else(|| Flow::Trap("byte-slice-byte-at out of bounds".into()))
    }

    pub(super) fn set_byte(
        &mut self,
        value: &EvalValue,
        index: usize,
        byte: u8,
    ) -> Result<(), Flow> {
        let loan = self.mutable_loan(value)?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(map_store_error)?;
        let bytes = self
            .store
            .byte_vector_range_mut(key, loan.start, loan.len)
            .map_err(map_store_error)?;
        let slot = bytes
            .get_mut(index)
            .ok_or_else(|| Flow::Trap("byte-slice-mut-set-byte out of bounds".into()))?;
        *slot = byte;
        Ok(())
    }

    fn shared_loan(&self, value: &EvalValue) -> Result<Loan, Flow> {
        let EvalValue::ByteSlice(token) = value else {
            return Err(Flow::Trap("expected exact byte-slice".into()));
        };
        self.loans
            .get(token)
            .copied()
            .filter(|loan| !loan.mutable)
            .ok_or_else(|| Flow::Trap("stale or wrong evaluator byte-slice".into()))
    }

    fn mutable_loan(&self, value: &EvalValue) -> Result<Loan, Flow> {
        let EvalValue::ByteSliceMut(token) = value else {
            return Err(Flow::Trap("expected exact byte-slice-mut".into()));
        };
        self.loans
            .get(token)
            .copied()
            .filter(|loan| loan.mutable)
            .ok_or_else(|| Flow::Trap("stale or wrong evaluator byte-slice-mut".into()))
    }
}

fn view_token(value: &EvalValue) -> Result<u64, Flow> {
    match value {
        EvalValue::BytesBorrow(token)
        | EvalValue::ByteSlice(token)
        | EvalValue::ByteSliceMut(token) => Ok(*token),
        _ => Err(Flow::Trap("expected exact byte view".into())),
    }
}
