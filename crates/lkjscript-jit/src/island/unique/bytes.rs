use super::*;

impl JitUniqueRuntime {
    pub(crate) fn clone_static_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<NativeUnique, NativeServiceError> {
        self.preflight_allocation(bytes.len())?;
        self.reserve_owner()?;
        let key = self
            .store
            .clone_static_bytes(bytes)
            .map_err(|error| self.store_error(error))?;
        self.publish_bytes_key(key.packed_word())
    }

    pub(crate) fn copy_static_bytes_slice(
        &mut self,
        bytes: &[u8],
        start: i64,
        len: i64,
    ) -> Result<NativeUnique, NativeServiceError> {
        let start = usize::try_from(start).map_err(|_| self.reject())?;
        let len = usize::try_from(len).map_err(|_| self.reject())?;
        self.preflight_allocation(len)?;
        self.reserve_owner()?;
        let key = self
            .store
            .clone_static_bytes_range(bytes, start, len)
            .map_err(|error| self.store_error(error))?;
        self.publish_bytes_key(key.packed_word())
    }

    pub(crate) fn thaw_static_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<NativeUnique, NativeServiceError> {
        self.preflight_allocation(bytes.len())?;
        self.reserve_owner()?;
        let key = self
            .store
            .thaw_bytes_slice(bytes)
            .map_err(|error| self.store_error(error))?;
        let word = key.packed_word();
        self.publish_owner(word.get())?;
        self.stats.allocations = self.stats.allocations.saturating_add(1);
        Ok(NativeUnique::byte_vector(word.get()))
    }

    pub(crate) fn move_bytes(
        &mut self,
        owner: NativeUnique,
    ) -> Result<NativeUnique, NativeServiceError> {
        let word = self.validate_owner(owner, lkjscript_native::UniqueType::Bytes)?;
        if self.active_loans_for(word).next().is_some() {
            return Err(self.reject());
        }
        self.stats.moves = self.stats.moves.saturating_add(1);
        Ok(owner)
    }

    pub(crate) fn bytes_length(&mut self, loan: NativeLoan) -> Result<i64, NativeServiceError> {
        if loan.loan_type() != LoanType::Bytes {
            return Err(self.reject());
        }
        let loan = self.loan(loan)?;
        self.stats.length_reads = self.stats.length_reads.saturating_add(1);
        i64::try_from(loan.len).map_err(|_| self.reject())
    }

    pub(crate) fn bytes_byte_at(
        &mut self,
        loan: NativeLoan,
        index: i64,
    ) -> Result<i64, NativeServiceError> {
        if loan.loan_type() != LoanType::Bytes {
            return Err(self.reject());
        }
        let index = usize::try_from(index).map_err(|_| self.reject())?;
        let loan = self.loan(loan)?;
        let key = self
            .store
            .import_bytes_key(loan.owner)
            .map_err(|_| self.reject())?;
        let byte = self
            .store
            .bytes_range(key, loan.start, loan.len)
            .ok()
            .and_then(|bytes| bytes.get(index).copied())
            .ok_or_else(|| self.reject())?;
        self.stats.byte_reads = self.stats.byte_reads.saturating_add(1);
        Ok(i64::from(byte))
    }

    pub(crate) fn clone_bytes(
        &mut self,
        loan: NativeLoan,
    ) -> Result<NativeUnique, NativeServiceError> {
        let loan = self.require_bytes_loan(loan)?;
        self.preflight_allocation(loan.len)?;
        self.reserve_owner()?;
        let key = self
            .store
            .import_bytes_key(loan.owner)
            .and_then(|key| self.store.clone_bytes(key))
            .map_err(|error| self.store_error(error))?;
        self.publish_bytes_key(key.packed_word())
    }

    pub(crate) fn copy_bytes_slice(
        &mut self,
        loan: NativeLoan,
        start: i64,
        len: i64,
    ) -> Result<NativeUnique, NativeServiceError> {
        let loan = self.require_bytes_loan(loan)?;
        let start = usize::try_from(start).map_err(|_| self.reject())?;
        let len = usize::try_from(len).map_err(|_| self.reject())?;
        self.preflight_allocation(len)?;
        self.reserve_owner()?;
        let key = self
            .store
            .import_bytes_key(loan.owner)
            .and_then(|key| self.store.clone_bytes_range(key, start, len))
            .map_err(|error| self.store_error(error))?;
        self.publish_bytes_key(key.packed_word())
    }

    pub(crate) fn freeze(
        &mut self,
        owner: NativeUnique,
    ) -> Result<NativeUnique, NativeServiceError> {
        let word = self.validate_owner(owner, lkjscript_native::UniqueType::ByteVector)?;
        if self.active_loans_for(word).next().is_some() {
            return Err(self.reject());
        }
        let key = self
            .store
            .import_byte_vector_key(word)
            .map_err(|_| self.reject())?;
        self.store
            .freeze_byte_vector(key)
            .map_err(|error| self.store_error(error))?;
        Ok(NativeUnique::bytes(word.get()))
    }

    pub(crate) fn thaw(&mut self, owner: NativeUnique) -> Result<NativeUnique, NativeServiceError> {
        let word = self.validate_owner(owner, lkjscript_native::UniqueType::Bytes)?;
        if self.active_loans_for(word).next().is_some() {
            return Err(self.reject());
        }
        let key = self
            .store
            .import_bytes_key(word)
            .map_err(|_| self.reject())?;
        self.store
            .thaw_dynamic_bytes(key)
            .map_err(|error| self.store_error(error))?;
        Ok(NativeUnique::byte_vector(word.get()))
    }

    fn require_bytes_loan(&mut self, loan: NativeLoan) -> Result<Loan, NativeServiceError> {
        if loan.loan_type() != LoanType::Bytes {
            return Err(self.reject());
        }
        self.loan(loan)
    }

    fn publish_bytes_key(
        &mut self,
        word: UniqueKeyWord,
    ) -> Result<NativeUnique, NativeServiceError> {
        self.publish_owner(word.get())?;
        self.stats.allocations = self.stats.allocations.saturating_add(1);
        Ok(NativeUnique::bytes(word.get()))
    }
}
