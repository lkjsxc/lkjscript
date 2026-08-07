use super::*;

impl JitUniqueRuntime {
    pub(crate) fn move_owner(
        &mut self,
        owner: NativeUnique,
    ) -> Result<NativeUnique, NativeServiceError> {
        let word = self.validate_owner(owner, lkjscript_native::UniqueType::ByteVector)?;
        if self.active_loans_for(word).next().is_some() {
            return Err(self.reject());
        }
        self.stats.moves = self.stats.moves.saturating_add(1);
        Ok(owner)
    }

    pub(crate) fn borrow(
        &mut self,
        owner: NativeUnique,
        kind: LoanType,
    ) -> Result<NativeLoan, NativeServiceError> {
        let owner_type = if kind == LoanType::Bytes {
            lkjscript_native::UniqueType::Bytes
        } else {
            lkjscript_native::UniqueType::ByteVector
        };
        let owner = self.validate_owner(owner, owner_type)?;
        let exclusive = kind == LoanType::ByteSliceMut;
        if self
            .active_loans_for(owner)
            .any(|loan| exclusive || loan.kind == LoanType::ByteSliceMut)
        {
            return Err(self.reject());
        }
        let len = match kind {
            LoanType::ByteSlice | LoanType::ByteSliceMut => {
                let key = self
                    .store
                    .import_byte_vector_key(owner)
                    .map_err(|_| self.reject())?;
                match self.store.byte_vector(key) {
                    Ok(bytes) => bytes.len(),
                    Err(_) => return Err(self.reject()),
                }
            }
            LoanType::Bytes => {
                let key = self
                    .store
                    .import_bytes_key(owner)
                    .map_err(|_| self.reject())?;
                match self.store.bytes(key) {
                    Ok(bytes) => bytes.len(),
                    Err(_) => return Err(self.reject()),
                }
            }
        };
        let loan = Loan {
            owner,
            kind,
            start: 0,
            len,
        };
        let token = self.install_loan(loan)?;
        match kind {
            LoanType::ByteSlice => {
                self.stats.shared_borrows = self.stats.shared_borrows.saturating_add(1);
                Ok(NativeLoan::byte_slice(token))
            }
            LoanType::ByteSliceMut => {
                self.stats.exclusive_borrows = self.stats.exclusive_borrows.saturating_add(1);
                Ok(NativeLoan::byte_slice_mut(token))
            }
            LoanType::Bytes => {
                self.stats.shared_borrows = self.stats.shared_borrows.saturating_add(1);
                Ok(NativeLoan::bytes(token))
            }
        }
    }

    pub(crate) fn length(&mut self, value: NativeLoan) -> Result<i64, NativeServiceError> {
        if value.loan_type() != LoanType::ByteSlice {
            return Err(self.reject());
        }
        let loan = self.loan(value)?;
        self.stats.length_reads = self.stats.length_reads.saturating_add(1);
        i64::try_from(loan.len).map_err(|_| self.reject())
    }

    pub(crate) fn byte_at(
        &mut self,
        value: NativeLoan,
        index: i64,
    ) -> Result<i64, NativeServiceError> {
        if value.loan_type() != LoanType::ByteSlice {
            return Err(self.reject());
        }
        let index = usize::try_from(index).map_err(|_| self.reject())?;
        let loan = self.loan(value)?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(|_| self.reject())?;
        let byte = match self.store.byte_vector_range(key, loan.start, loan.len) {
            Ok(bytes) => bytes.get(index).copied(),
            Err(_) => None,
        };
        let Some(byte) = byte else {
            return Err(self.reject());
        };
        self.stats.byte_reads = self.stats.byte_reads.saturating_add(1);
        Ok(i64::from(byte))
    }

    pub(crate) fn set_byte(
        &mut self,
        value: NativeLoan,
        index: i64,
        byte: i64,
    ) -> Result<(), NativeServiceError> {
        if value.loan_type() != LoanType::ByteSliceMut {
            return Err(self.reject());
        }
        let index = usize::try_from(index).map_err(|_| self.reject())?;
        let byte = u8::try_from(byte).map_err(|_| self.reject())?;
        let loan = self.loan(value)?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(|_| self.reject())?;
        let updated = match self.store.byte_vector_range_mut(key, loan.start, loan.len) {
            Ok(bytes) => bytes.get_mut(index).map(|slot| *slot = byte).is_some(),
            Err(_) => false,
        };
        if !updated {
            return Err(self.reject());
        }
        self.stats.byte_writes = self.stats.byte_writes.saturating_add(1);
        Ok(())
    }

    pub(crate) fn end_borrow(&mut self, value: NativeLoan) -> Result<(), NativeServiceError> {
        self.loan(value)?;
        let identity = self
            .loan_tokens
            .get(&value.opaque_word())
            .copied()
            .ok_or_else(|| self.reject())?;
        let index = usize::try_from(identity.slot).map_err(|_| self.reject())?;
        let Some(slot) = self.loans.get_mut(index) else {
            return Err(NativeServiceError::Trap);
        };
        slot.loan = None;
        slot.token = None;
        self.loan_tokens.remove(&value.opaque_word());
        self.stats.loan_ends = self.stats.loan_ends.saturating_add(1);
        Ok(())
    }

    pub(super) fn install_loan(&mut self, loan: Loan) -> Result<u64, NativeServiceError> {
        let active = self.loans.iter().filter(|slot| slot.loan.is_some()).count();
        if self.max_loans.is_some_and(|maximum| active >= maximum) {
            return Err(self.loan_limit());
        }
        self.loan_tokens
            .try_reserve(1)
            .map_err(|_| NativeServiceError::HostFailure)?;
        let token = self
            .next_loan_token
            .ok_or(NativeServiceError::HostFailure)?;
        self.next_loan_token = token
            .get()
            .checked_add(1)
            .and_then(std::num::NonZeroU64::new);
        let reusable = self
            .loans
            .iter()
            .position(|slot| slot.loan.is_none() && slot.generation.get() < u64::MAX);
        let (slot_index, generation) = if let Some(index) = reusable {
            let slot_index = u64::try_from(index).map_err(|_| NativeServiceError::HostFailure)?;
            let generation = self.loans[index]
                .generation
                .get()
                .checked_add(1)
                .and_then(std::num::NonZeroU64::new)
                .ok_or(NativeServiceError::HostFailure)?;
            (slot_index, generation)
        } else {
            self.loans
                .try_reserve(1)
                .map_err(|_| NativeServiceError::HostFailure)?;
            let slot_index =
                u64::try_from(self.loans.len()).map_err(|_| NativeServiceError::HostFailure)?;
            (slot_index, std::num::NonZeroU64::MIN)
        };
        let identity = LoanIdentity {
            slot: slot_index,
            generation,
        };
        if self.loan_tokens.contains_key(&token.get()) {
            return Err(NativeServiceError::HostFailure);
        }
        let index = usize::try_from(slot_index).map_err(|_| NativeServiceError::HostFailure)?;
        if reusable.is_some() {
            self.loans[index] = LoanSlot {
                generation,
                token: Some(token),
                loan: Some(loan),
            };
        } else {
            self.loans.push(LoanSlot {
                generation,
                token: Some(token),
                loan: Some(loan),
            });
        }
        self.loan_tokens.insert(token.get(), identity);
        Ok(token.get())
    }
}
