use std::collections::BTreeMap;

use lkjscript_core::{UniqueKeyWord, UniqueLayout, UniqueStore, UniqueStoreId};

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
    max_allocations: u64,
    max_heap_bytes: usize,
}

impl EvalUniqueRuntime {
    pub(super) fn new(config: &EvalConfig) -> Option<Self> {
        let id = UniqueStoreId::new(1)?;
        Some(Self {
            store: UniqueStore::new(id),
            owners: BTreeMap::new(),
            loans: BTreeMap::new(),
            next_loan: 1,
            max_allocations: config.max_allocations,
            max_heap_bytes: config.max_heap_bytes,
        })
    }

    pub(super) fn allocate(&mut self, size: usize) -> Result<EvalValue, Flow> {
        self.preflight_allocation(size)?;
        self.store
            .check_byte_vector_allocation(size)
            .map_err(map_store_error)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| Flow::HostFailure("evaluator byte-vector allocation failed".into()))?;
        bytes.resize(size, 0);
        let key = self
            .store
            .allocate_byte_vector(bytes)
            .map_err(map_store_error)?;
        let word = key.opaque_word();
        if self
            .owners
            .insert(word.get(), UniqueLayout::ByteVector)
            .is_some()
        {
            return Err(Flow::Trap("duplicate evaluator byte-vector owner".into()));
        }
        Ok(EvalValue::ByteVector(word))
    }

    fn preflight_allocation(&self, bytes: usize) -> Result<(), Flow> {
        let stats = self.store.stats();
        if stats.allocations >= self.max_allocations {
            return Err(Flow::Resource("allocations".into()));
        }
        let bytes = u64::try_from(bytes)
            .map_err(|_| Flow::HostFailure("evaluator unique byte count exceeds u64".into()))?;
        let projected = stats
            .live_bytes
            .checked_add(bytes)
            .ok_or_else(|| Flow::HostFailure("evaluator unique heap accounting overflow".into()))?;
        let maximum = u64::try_from(self.max_heap_bytes).unwrap_or(u64::MAX);
        if projected > maximum {
            return Err(Flow::Resource("heap bytes".into()));
        }
        Ok(())
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn evaluator_byte_vector_uses_explicit_heap_policy_beyond_former_limit() {
        let size = 1_000_001;
        let low = EvalConfig {
            max_heap_bytes: size - 1,
            ..EvalConfig::default()
        };
        let mut limited = EvalUniqueRuntime::new(&low).expect("limited runtime");
        assert!(matches!(limited.allocate(size), Err(Flow::Resource(_))));

        let high = EvalConfig {
            max_heap_bytes: size * 2,
            ..EvalConfig::default()
        };
        let mut runtime = EvalUniqueRuntime::new(&high).expect("high-policy runtime");
        let owner = runtime.allocate(size).expect("large evaluator vector");
        let view = runtime.borrow(&owner, false).expect("borrow large vector");
        assert_eq!(
            runtime.len(&view).ok(),
            Some(i64::try_from(size).expect("test size fits i64"))
        );
        runtime.end_borrow(view).expect("end large borrow");
        runtime.drop_owner(owner).expect("drop large vector");
        runtime.verify_empty().expect("empty runtime");
    }
}
