use crate::island::config_error;
use crate::*;

mod access;
mod bytes;
mod cleanup;
#[cfg(test)]
mod tests;
mod word;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Loan {
    owner: UniqueKeyWord,
    kind: LoanType,
    start: usize,
    len: usize,
}

#[derive(Clone, Copy, Debug)]
struct LoanSlot {
    generation: std::num::NonZeroU64,
    token: Option<std::num::NonZeroU64>,
    loan: Option<Loan>,
}

#[derive(Clone, Copy, Debug)]
struct LoanIdentity {
    slot: u64,
    generation: std::num::NonZeroU64,
}

pub(super) struct JitUniqueRuntime {
    store: UniqueStore,
    owners: Vec<u64>,
    loans: Vec<LoanSlot>,
    loan_tokens: std::collections::HashMap<u64, LoanIdentity>,
    next_loan_token: Option<std::num::NonZeroU64>,
    max_loans: Option<usize>,
    max_allocations: Option<u64>,
    max_heap_bytes: Option<u64>,
    stats: NativeUniqueStats,
    last_resource: Option<ResourceLimitKind>,
}

impl JitUniqueRuntime {
    pub(super) fn new(config: &ExecutionPolicy) -> Result<Self, EngineError> {
        let id = UniqueStoreId::new(1).ok_or_else(config_error)?;
        Ok(Self {
            store: UniqueStore::new(id),
            owners: Vec::new(),
            loans: Vec::new(),
            loan_tokens: std::collections::HashMap::new(),
            next_loan_token: std::num::NonZeroU64::new(1),
            max_loans: config.max_handles(),
            max_allocations: config.max_allocations(),
            max_heap_bytes: config
                .max_heap_bytes()
                .and_then(|bytes| u64::try_from(bytes).ok()),
            stats: NativeUniqueStats::default(),
            last_resource: None,
        })
    }

    pub(super) fn allocate(&mut self, size: i64) -> Result<NativeUnique, NativeServiceError> {
        let size = usize::try_from(size).map_err(|_| self.reject())?;
        let next_allocations = self.preflight_allocation(size)?;
        if let Err(error) = self.store.check_byte_vector_allocation(size) {
            return Err(self.store_error(error));
        }
        self.owners
            .try_reserve(1)
            .map_err(|_| NativeServiceError::HostFailure)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| NativeServiceError::HostFailure)?;
        bytes.resize(size, 0);
        let key = self
            .store
            .allocate_byte_vector(bytes)
            .map_err(|error| self.store_error(error))?;
        let word = key.opaque_word().get();
        self.publish_owner(word)?;
        self.stats.allocations = next_allocations;
        Ok(NativeUnique::byte_vector(word))
    }

    fn validate_owner(
        &mut self,
        owner: NativeUnique,
        expected: lkjscript_native::UniqueType,
    ) -> Result<UniqueKeyWord, NativeServiceError> {
        if owner.unique_type() != expected || !self.owners.contains(&owner.opaque_word()) {
            return Err(self.reject());
        }
        let word = UniqueKeyWord::new(owner.opaque_word()).map_err(|_| self.reject())?;
        let valid = match expected {
            lkjscript_native::UniqueType::ByteVector => {
                self.store.import_byte_vector_key(word).is_ok()
            }
            lkjscript_native::UniqueType::Bytes => self.store.import_bytes_key(word).is_ok(),
        };
        if !valid {
            return Err(self.reject());
        }
        Ok(word)
    }

    fn reserve_owner(&mut self) -> Result<(), NativeServiceError> {
        self.owners
            .try_reserve(1)
            .map_err(|_| NativeServiceError::HostFailure)
    }

    fn preflight_allocation(&mut self, bytes: usize) -> Result<u64, NativeServiceError> {
        let next_allocations = self
            .stats
            .allocations
            .checked_add(1)
            .ok_or(NativeServiceError::HostFailure)?;
        if self
            .max_allocations
            .is_some_and(|maximum| next_allocations > maximum)
        {
            self.last_resource = Some(ResourceLimitKind::Allocations);
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        let bytes = u64::try_from(bytes).map_err(|_| NativeServiceError::HostFailure)?;
        let projected = self
            .store
            .stats()
            .live_bytes
            .checked_add(bytes)
            .ok_or(NativeServiceError::HostFailure)?;
        if self
            .max_heap_bytes
            .is_some_and(|maximum| projected > maximum)
        {
            return Err(self.heap_limit());
        }
        Ok(next_allocations)
    }

    fn publish_owner(&mut self, word: u64) -> Result<(), NativeServiceError> {
        if self.owners.contains(&word) {
            return Err(self.reject());
        }
        self.owners.push(word);
        Ok(())
    }

    fn active_loans_for(&self, owner: UniqueKeyWord) -> impl Iterator<Item = Loan> + '_ {
        self.loans
            .iter()
            .filter_map(|slot| slot.loan)
            .filter(move |loan| loan.owner == owner)
    }

    fn loan(&mut self, value: NativeLoan) -> Result<Loan, NativeServiceError> {
        let word = value.opaque_word();
        let identity = self
            .loan_tokens
            .get(&word)
            .copied()
            .ok_or_else(|| self.reject())?;
        let index = usize::try_from(identity.slot).map_err(|_| self.reject())?;
        let loan = self
            .loans
            .get(index)
            .filter(|slot| {
                slot.token.is_some_and(|token| token.get() == word)
                    && slot.generation == identity.generation
            })
            .and_then(|slot| slot.loan)
            .filter(|loan| loan.kind == value.loan_type())
            .ok_or_else(|| self.reject())?;
        let valid = match loan.kind {
            LoanType::ByteSlice | LoanType::ByteSliceMut => self
                .store
                .import_byte_vector_key(loan.owner)
                .and_then(|key| self.store.byte_vector_range(key, loan.start, loan.len))
                .is_ok(),
            LoanType::Bytes => self
                .store
                .import_bytes_key(loan.owner)
                .and_then(|key| self.store.bytes_range(key, loan.start, loan.len))
                .is_ok(),
        };
        if !valid {
            return Err(self.reject());
        }
        Ok(loan)
    }

    fn reject(&mut self) -> NativeServiceError {
        self.stats.stale_or_forged_failures = self.stats.stale_or_forged_failures.saturating_add(1);
        NativeServiceError::Trap
    }

    fn heap_limit(&mut self) -> NativeServiceError {
        self.last_resource = Some(ResourceLimitKind::HeapBytes);
        NativeServiceError::ResourceLimitExceeded
    }

    fn loan_limit(&mut self) -> NativeServiceError {
        self.last_resource = Some(ResourceLimitKind::Handles);
        NativeServiceError::ResourceLimitExceeded
    }

    fn store_error(&mut self, error: UniqueStoreError) -> NativeServiceError {
        match error {
            UniqueStoreError::RepresentationExhausted
            | UniqueStoreError::ArithmeticOverflow
            | UniqueStoreError::StorageCapacity => NativeServiceError::HostFailure,
            _ => self.reject(),
        }
    }
}
