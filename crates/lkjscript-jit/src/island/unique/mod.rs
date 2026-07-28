use crate::island::{config_error, EngineErrorDetail};
use crate::*;

mod access;
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
    generation: u32,
    loan: Option<Loan>,
}

pub(super) struct JitUniqueRuntime {
    store: UniqueStore,
    owners: Vec<u64>,
    loans: Vec<LoanSlot>,
    max_loans: usize,
    stats: NativeUniqueStats,
    last_resource: Option<ResourceLimitKind>,
}

impl JitUniqueRuntime {
    pub(super) fn new(config: &ExecutionConfig) -> Result<Self, EngineError> {
        let objects = u32::try_from(config.max_allocations).unwrap_or(u32::MAX);
        let bytes = u64::try_from(config.max_heap_bytes).unwrap_or(u64::MAX);
        let id = UniqueStoreId::new(1).ok_or_else(config_error)?;
        let limits =
            UniqueStoreLimits::new(objects, bytes, objects, config.max_allocations, u32::MAX)
                .map_err(|error| config_error().with_detail(error.to_string()))?;
        Ok(Self {
            store: UniqueStore::new(id, limits),
            owners: Vec::new(),
            loans: Vec::new(),
            max_loans: config.max_stack_values.min(u32::MAX as usize),
            stats: NativeUniqueStats::default(),
            last_resource: None,
        })
    }

    pub(super) fn allocate(&mut self, size: i64) -> Result<NativeUnique, NativeServiceError> {
        let size = usize::try_from(size)
            .ok()
            .filter(|size| *size <= MAX_BUFFER_BYTES)
            .ok_or_else(|| self.reject())?;
        self.owners.try_reserve(1).map_err(|_| self.heap_limit())?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| self.heap_limit())?;
        bytes.resize(size, 0);
        let key = self
            .store
            .allocate_byte_vector(bytes)
            .map_err(|error| self.store_error(error))?;
        let word = key.packed_word().get();
        if self.owners.contains(&word) {
            return Err(self.reject());
        }
        self.owners.push(word);
        self.stats.allocations = self.stats.allocations.saturating_add(1);
        Ok(NativeUnique::byte_vector(word))
    }

    fn validate_owner(&mut self, owner: NativeUnique) -> Result<UniqueKeyWord, NativeServiceError> {
        if owner.unique_type() != lkjscript_native::UniqueType::ByteVector
            || !self.owners.contains(&owner.opaque_word())
        {
            return Err(self.reject());
        }
        let word = UniqueKeyWord::new(owner.opaque_word()).map_err(|_| self.reject())?;
        self.store
            .import_byte_vector_key(word)
            .map_err(|_| self.reject())?;
        Ok(word)
    }

    fn active_loans_for(&self, owner: UniqueKeyWord) -> impl Iterator<Item = Loan> + '_ {
        self.loans
            .iter()
            .filter_map(|slot| slot.loan)
            .filter(move |loan| loan.owner == owner)
    }

    fn loan(&mut self, value: NativeLoan) -> Result<Loan, NativeServiceError> {
        let word = value.opaque_word();
        let generation = u32::try_from(word >> 32).map_err(|_| self.reject())?;
        let index = word as u32 as usize;
        let loan = self
            .loans
            .get(index)
            .filter(|slot| generation != 0 && slot.generation == generation)
            .and_then(|slot| slot.loan)
            .filter(|loan| loan.kind == value.loan_type())
            .ok_or_else(|| self.reject())?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(|_| self.reject())?;
        if self
            .store
            .byte_vector_range(key, loan.start, loan.len)
            .is_err()
        {
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
        self.last_resource = Some(ResourceLimitKind::StackValues);
        NativeServiceError::ResourceLimitExceeded
    }

    fn store_error(&mut self, error: UniqueStoreError) -> NativeServiceError {
        match error {
            UniqueStoreError::AllocationLimit | UniqueStoreError::ObjectLimit => {
                self.last_resource = Some(ResourceLimitKind::Allocations);
                NativeServiceError::ResourceLimitExceeded
            }
            UniqueStoreError::ByteLimit
            | UniqueStoreError::SlotLimit
            | UniqueStoreError::StorageCapacity => self.heap_limit(),
            _ => self.reject(),
        }
    }
}
