use std::num::NonZeroU64;

use super::slot::Binding;
use super::{ResourceKey, ResourceObservation, ResourceTable, ResourceTableStats};

pub struct OwnedReservation<'a, P> {
    pub(super) table: &'a mut ResourceTable<P>,
    pub(super) slot: usize,
    pub(super) generation: NonZeroU64,
    pub(super) binding: Binding,
    pub(super) sequence: u64,
    pub(super) active: bool,
}

impl<P> OwnedReservation<'_, P> {
    pub fn observation(&self) -> ResourceObservation {
        self.table.slots[self.slot].observation()
    }

    pub fn stats(&self) -> ResourceTableStats {
        self.table.stats()
    }

    pub fn commit(mut self, payload: P) -> ResourceKey {
        let key = self.table.commit_reservation(
            self.slot,
            self.generation,
            self.binding,
            self.sequence,
            payload,
        );
        self.active = false;
        key
    }

    pub fn acquire<E>(self, acquire: impl FnOnce() -> Result<P, E>) -> Result<ResourceKey, E> {
        match acquire() {
            Ok(payload) => Ok(self.commit(payload)),
            Err(error) => Err(error),
        }
    }

    pub fn cancel(mut self) {
        self.table.cancel_reservation(self.slot, self.binding);
        self.active = false;
    }
}

impl<P> Drop for OwnedReservation<'_, P> {
    fn drop(&mut self) {
        if self.active {
            self.table.cancel_reservation(self.slot, self.binding);
            self.active = false;
        }
    }
}

pub struct BorrowedReservation<'a, P> {
    pub(super) table: &'a mut ResourceTable<P>,
    pub(super) slot: usize,
    pub(super) generation: NonZeroU64,
    pub(super) binding: Binding,
    pub(super) sequence: u64,
    pub(super) active: bool,
}

impl<P> BorrowedReservation<'_, P> {
    pub fn observation(&self) -> ResourceObservation {
        self.table.slots[self.slot].observation()
    }

    pub fn stats(&self) -> ResourceTableStats {
        self.table.stats()
    }

    pub fn commit(mut self, payload: P) -> ResourceKey {
        let key = self.table.commit_reservation(
            self.slot,
            self.generation,
            self.binding,
            self.sequence,
            payload,
        );
        self.active = false;
        key
    }

    pub fn acquire<E>(self, acquire: impl FnOnce() -> Result<P, E>) -> Result<ResourceKey, E> {
        match acquire() {
            Ok(payload) => Ok(self.commit(payload)),
            Err(error) => Err(error),
        }
    }

    pub fn cancel(mut self) {
        self.table.cancel_reservation(self.slot, self.binding);
        self.active = false;
    }
}

impl<P> Drop for BorrowedReservation<'_, P> {
    fn drop(&mut self) {
        if self.active {
            self.table.cancel_reservation(self.slot, self.binding);
            self.active = false;
        }
    }
}
