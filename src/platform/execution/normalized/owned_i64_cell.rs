//! A sealed affine scalar allocation, independent of byte-buffer storage.
use super::value::ValueOrigin;
use crate::platform::execution::{ExecutionControl, ExecutionError};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Owner,
    Read,
    Inert,
}

#[derive(Debug)]
struct Storage {
    scalar: Option<i64>,
    loans: usize,
}

pub struct OwnedI64Cell {
    domain: ValueOrigin,
    state: Arc<Mutex<Storage>>,
    mode: Mode,
}

impl Clone for OwnedI64Cell {
    fn clone(&self) -> Self {
        Self {
            domain: self.domain,
            state: Arc::clone(&self.state),
            mode: Mode::Inert,
        }
    }
}

impl PartialEq for OwnedI64Cell {
    fn eq(&self, other: &Self) -> bool {
        self.domain == other.domain && Arc::ptr_eq(&self.state, &other.state)
    }
}
impl Eq for OwnedI64Cell {}
impl std::fmt::Debug for OwnedI64Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OwnedI64Cell(<opaque token>)")
    }
}

fn reject() -> ExecutionError {
    ExecutionError::resource(
        "normalized_cell_token",
        "OwnedI64Cell token is stale, foreign, borrowed, or unadmitted",
    )
}

impl OwnedI64Cell {
    #[cfg(test)]
    pub(super) fn allocation_identity(&self) -> usize {
        Arc::as_ptr(&self.state) as usize
    }
    pub(super) const ALLOCATION_BYTES: u64 = (std::mem::size_of::<Mutex<Storage>>()
        + 2 * std::mem::size_of::<usize>()
        + std::mem::size_of::<Self>()) as u64;

    fn lock(&self) -> MutexGuard<'_, Storage> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    // The evaluator must reserve ALLOCATION_BYTES before calling this constructor.
    pub(super) fn new(domain: ValueOrigin, scalar: i64) -> Self {
        let cell = Self {
            domain,
            state: Arc::new(Mutex::new(Storage {
                scalar: Some(scalar),
                loans: 0,
            })),
            mode: Mode::Owner,
        };
        #[cfg(test)]
        OBSERVED.with(|entries| {
            if let Some(entries) = entries.borrow_mut().as_mut() {
                entries.push(Arc::downgrade(&cell.state));
            }
        });
        cell
    }

    pub(super) fn validate(
        &self,
        domain: ValueOrigin,
        consume: bool,
    ) -> Result<(), ExecutionError> {
        let storage = self.lock();
        if self.domain != domain
            || self.mode == Mode::Inert
            || storage.scalar.is_none()
            || (consume && (self.mode != Mode::Owner || storage.loans != 0))
        {
            return Err(reject());
        }
        Ok(())
    }

    pub(super) fn is_borrowed(&self) -> bool {
        self.mode == Mode::Read
    }
    pub(super) fn owns_live_loans(&self) -> bool {
        self.mode == Mode::Owner && self.lock().loans != 0
    }
    pub(super) fn borrow(&self) -> Result<Self, ExecutionError> {
        self.validate(self.domain, false)?;
        let mut storage = self.lock();
        storage.loans = storage.loans.checked_add(1).ok_or_else(reject)?;
        Ok(Self {
            domain: self.domain,
            state: Arc::clone(&self.state),
            mode: Mode::Read,
        })
    }
    pub(super) fn read(&self) -> Result<i64, ExecutionError> {
        self.validate(self.domain, false)?;
        self.lock().scalar.ok_or_else(reject)
    }
    pub(super) fn replace(
        self,
        scalar: i64,
        control: &ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        self.validate(self.domain, true)?;
        control.check()?;
        self.lock().scalar = Some(scalar);
        control.check()?;
        Ok(self)
    }
    pub(super) fn extract(self) -> Result<i64, ExecutionError> {
        self.validate(self.domain, true)?;
        self.lock().scalar.take().ok_or_else(reject)
    }
}

impl Drop for OwnedI64Cell {
    fn drop(&mut self) {
        let mut storage = self.lock();
        match self.mode {
            Mode::Owner => {
                storage.scalar.take();
            }
            Mode::Read => {
                storage.loans = storage.loans.saturating_sub(1);
            }
            Mode::Inert => {}
        }
    }
}

#[cfg(test)]
thread_local! {static OBSERVED:std::cell::RefCell<Option<Vec<std::sync::Weak<Mutex<Storage>>>>> = const { std::cell::RefCell::new(None) };}
#[cfg(test)]
pub(super) struct StorageObservation;
#[cfg(test)]
impl StorageObservation {
    pub(super) fn start() -> Self {
        OBSERVED.with(|entries| {
            assert!(entries.borrow().is_none());
            *entries.borrow_mut() = Some(Vec::new());
        });
        Self
    }
    pub(super) fn live(&self) -> (usize, usize) {
        OBSERVED.with(|entries| {
            entries
                .borrow()
                .as_ref()
                .unwrap()
                .iter()
                .filter_map(std::sync::Weak::upgrade)
                .map(|s| {
                    let s = s.lock().unwrap();
                    (usize::from(s.scalar.is_some()), s.loans)
                })
                .fold((0, 0), |(a, b), (x, y)| (a + x, b + y))
        })
    }
}
#[cfg(test)]
impl Drop for StorageObservation {
    fn drop(&mut self) {
        OBSERVED.with(|entries| {
            entries.borrow_mut().take();
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_cell_seals_origin_and_clone_and_cleans_failed_transfer() {
        let domain = ValueOrigin::fresh().unwrap();
        let owner = OwnedI64Cell::new(domain, i64::MIN);
        let marker = owner.clone();
        assert!(marker.read().is_err());
        assert!(owner.validate(ValueOrigin::fresh().unwrap(), true).is_err());
        let loan = owner.borrow().unwrap();
        let reborrow = loan.borrow().unwrap();
        assert_eq!(reborrow.read().unwrap(), i64::MIN);
        assert!(owner.validate(domain, true).is_err());
        assert!(loan.validate(domain, true).is_err());
        drop(reborrow);
        drop(loan);
        let owner = owner
            .replace(i64::MAX, &ExecutionControl::uncancelled())
            .unwrap();
        assert_eq!(owner.extract().unwrap(), i64::MAX);
        assert!(marker.lock().scalar.is_none());
        let owner = OwnedI64Cell::new(domain, -257);
        let marker = owner.clone();
        let control = ExecutionControl::uncancelled();
        control.cancel();
        assert!(owner.replace(0, &control).is_err());
        assert!(marker.lock().scalar.is_none());
        let owner = OwnedI64Cell::new(domain, 128);
        let loan = owner.borrow().unwrap();
        drop(owner);
        assert!(loan.read().is_err());
    }
}
