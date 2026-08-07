use std::marker::PhantomData;

use super::model::ObjectLocation;
use super::{
    SealedBorrow, SealedOwner, SealedRef, SealedRegionStore, SealedUpgrade, WeakSealedRef,
};
use crate::structural::StructuralError;

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub fn retain(
        &mut self,
        owner: &SealedOwner<T, D>,
    ) -> Result<SealedOwner<T, D>, StructuralError> {
        let record = self.record_mut(owner.key)?;
        let owners = record
            .owners
            .checked_add(1)
            .ok_or(StructuralError::OwnerOverflow)?;
        record.owners = owners;
        self.metrics.retains = self.metrics.retains.saturating_add(1);
        Ok(SealedOwner {
            key: owner.key,
            marker: PhantomData,
        })
    }

    pub const fn downgrade(&self, reference: SealedRef<T>) -> WeakSealedRef<T> {
        WeakSealedRef {
            key: reference.key,
            marker: PhantomData,
        }
    }

    pub fn upgrade(
        &mut self,
        weak: WeakSealedRef<T>,
    ) -> Result<Option<SealedUpgrade<T, D>>, StructuralError> {
        if weak.key.layout() != self.layout || weak.key.semantic_type() != self.semantic_type {
            return Err(StructuralError::WrongLayout);
        }
        let Ok(index) = self.record_index(weak.key.domain()) else {
            return Ok(None);
        };
        let record = &mut self.records[index].1;
        if record.owners == 0
            || usize::try_from(weak.key.slot()).map_or(true, |slot| slot >= record.roots.len())
            || weak.key.generation() != weak.key.domain().generation()
        {
            return Ok(None);
        }
        let owners = record
            .owners
            .checked_add(1)
            .ok_or(StructuralError::OwnerOverflow)?;
        record.owners = owners;
        self.metrics.weak_upgrades = self.metrics.weak_upgrades.saturating_add(1);
        Ok(Some((
            SealedOwner {
                key: weak.key.domain(),
                marker: PhantomData,
            },
            SealedRef {
                key: weak.key,
                marker: PhantomData,
            },
        )))
    }

    pub fn begin_borrow(
        &mut self,
        reference: SealedRef<T>,
    ) -> Result<SealedBorrow<T>, StructuralError> {
        self.get(reference)?;
        let record = self.record_mut(reference.key.domain())?;
        record.loans = record
            .loans
            .checked_add(1)
            .ok_or(StructuralError::ArithmeticOverflow)?;
        Ok(SealedBorrow {
            key: reference.key,
            marker: PhantomData,
        })
    }

    pub fn borrowed(&self, borrow: &SealedBorrow<T>) -> Result<&T, StructuralError> {
        self.get(SealedRef {
            key: borrow.key,
            marker: PhantomData,
        })
    }

    pub fn borrowed_at(&self, borrow: &SealedBorrow<T>, slot: u64) -> Result<&T, StructuralError> {
        self.borrowed(borrow)?;
        let index = self.record_index(borrow.key.domain())?;
        let record = &self.records[index].1;
        if record.loans == 0 {
            return Err(StructuralError::LiveLoan);
        }
        let root = record
            .roots
            .get(usize::try_from(slot).map_err(|_| StructuralError::ArithmeticOverflow)?)
            .ok_or(StructuralError::StaleRoot(borrow.key))?;
        if root.generation != borrow.key.generation() {
            return Err(StructuralError::StaleRoot(borrow.key));
        }
        match root.location {
            ObjectLocation::Chunk { chunk, offset } => record
                .chunks
                .get(usize::try_from(chunk).map_err(|_| StructuralError::ArithmeticOverflow)?)
                .and_then(|values| {
                    usize::try_from(offset)
                        .ok()
                        .and_then(|offset| values.get(offset))
                })
                .ok_or(StructuralError::StaleRoot(borrow.key)),
            ObjectLocation::Large { index } => record
                .large
                .get(usize::try_from(index).map_err(|_| StructuralError::ArithmeticOverflow)?)
                .and_then(|values| values.first())
                .ok_or(StructuralError::StaleRoot(borrow.key)),
        }
    }

    #[allow(clippy::result_large_err)]
    pub fn end_borrow(
        &mut self,
        borrow: SealedBorrow<T>,
    ) -> Result<(), (StructuralError, SealedBorrow<T>)> {
        let record = match self.record_mut(borrow.key.domain()) {
            Ok(record) => record,
            Err(error) => return Err((error, borrow)),
        };
        let Some(loans) = record.loans.checked_sub(1) else {
            return Err((StructuralError::LoanUnderflow, borrow));
        };
        record.loans = loans;
        Ok(())
    }
}
