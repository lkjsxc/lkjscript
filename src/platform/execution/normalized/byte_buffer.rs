//! Sealed memory tokens. Arc shares only liveness metadata; one owner controls the Vec.
use super::bytes::BytePayload;
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
    bytes: Option<Vec<u8>>,
    loans: usize,
}
pub struct ByteBuffer {
    domain: ValueOrigin,
    state: Arc<Mutex<Storage>>,
    mode: Mode,
}
// Raw Clone NEVER duplicates ownership or manufactures a loan.
impl Clone for ByteBuffer {
    fn clone(&self) -> Self {
        Self {
            domain: self.domain,
            state: Arc::clone(&self.state),
            mode: Mode::Inert,
        }
    }
}
impl PartialEq for ByteBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.domain == other.domain && Arc::ptr_eq(&self.state, &other.state)
    }
}
impl Eq for ByteBuffer {}
impl std::fmt::Debug for ByteBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ByteBuffer(<opaque token>)")
    }
}
fn reject() -> ExecutionError {
    ExecutionError::resource(
        "normalized_buffer_token",
        "ByteBuffer token is stale, foreign, borrowed, or unadmitted",
    )
}
impl ByteBuffer {
    fn lock(&self) -> MutexGuard<'_, Storage> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(super) fn empty(domain: ValueOrigin) -> Self {
        let buffer = Self {
            domain,
            state: Arc::new(Mutex::new(Storage {
                bytes: Some(Vec::new()),
                loans: 0,
            })),
            mode: Mode::Owner,
        };
        #[cfg(test)]
        OBSERVED.with(|entries| {
            if let Some(entries) = entries.borrow_mut().as_mut() {
                entries.push(Arc::downgrade(&buffer.state));
            }
        });
        buffer
    }
    pub(super) fn validate(
        &self,
        domain: ValueOrigin,
        consume: bool,
    ) -> Result<(), ExecutionError> {
        let s = self.lock();
        if self.domain != domain
            || self.mode == Mode::Inert
            || s.bytes.is_none()
            || (consume && (self.mode != Mode::Owner || s.loans != 0))
        {
            return Err(reject());
        }
        Ok(())
    }
    pub(super) fn owns_live_loans(&self) -> bool {
        self.mode == Mode::Owner && self.lock().loans != 0
    }
    pub(super) fn is_borrowed(&self) -> bool {
        self.mode == Mode::Read
    }
    pub(super) fn borrow(&self) -> Result<Self, ExecutionError> {
        self.validate(self.domain, false)?;
        let mut s = self.lock();
        s.loans = s.loans.checked_add(1).ok_or_else(reject)?;
        Ok(Self {
            domain: self.domain,
            state: Arc::clone(&self.state),
            mode: Mode::Read,
        })
    }
    pub(super) fn len(&self) -> Result<usize, ExecutionError> {
        self.validate(self.domain, false)?;
        self.lock().bytes.as_ref().map(Vec::len).ok_or_else(reject)
    }
    pub(super) fn get(&self, index: i64) -> Result<u8, ExecutionError> {
        self.validate(self.domain, false)?;
        usize::try_from(index)
            .ok()
            .and_then(|i| self.lock().bytes.as_ref().and_then(|v| v.get(i)).copied())
            .ok_or_else(|| {
                ExecutionError::new(
                    crate::platform::execution::ExecutionFailureClass::Trap,
                    "normalized_buffer_index",
                    "ByteBuffer index is outside its octets",
                )
            })
    }
    pub(super) fn push(
        self,
        octet: i64,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        self.validate(self.domain, true)?;
        control.check()?;
        let octet = u8::try_from(octet).map_err(|_| {
            ExecutionError::new(
                crate::platform::execution::ExecutionFailureClass::Trap,
                "normalized_buffer_octet",
                "ByteBuffer octet must be in 0 through 255",
            )
        })?;
        {
            let mut s = self.lock();
            let v = s.bytes.as_mut().ok_or_else(reject)?;
            let length = v.len().checked_add(1).ok_or_else(reject)?;
            if length as u64 > super::value::MAXIMUM_VALUE_ALLOCATION_BYTES {
                return Err(ExecutionError::resource(
                    "normalized_buffer_storage",
                    "ByteBuffer exceeds finite storage",
                ));
            }
            if length > v.capacity() {
                let capacity = v
                    .capacity()
                    .saturating_mul(2)
                    .max(8)
                    .max(length)
                    .min(super::value::MAXIMUM_VALUE_ALLOCATION_BYTES as usize);
                reserve(capacity as u64)?;
                v.try_reserve_exact(capacity - v.len()).map_err(|_| {
                    ExecutionError::resource(
                        "normalized_buffer_storage",
                        "cannot allocate admitted ByteBuffer storage",
                    )
                })?;
            }
            control.check()?;
            v.push(octet);
        }
        control.check()?;
        Ok(self)
    }
    pub(super) fn freeze(self) -> Result<BytePayload, ExecutionError> {
        self.validate(self.domain, true)?;
        let bytes = self.lock().bytes.take().ok_or_else(reject)?;
        Ok(BytePayload::adopt_vec(bytes))
    }
}
impl Drop for ByteBuffer {
    fn drop(&mut self) {
        let mut s = self.lock();
        match self.mode {
            Mode::Owner => {
                s.bytes.take();
            }
            Mode::Read => {
                s.loans = s.loans.saturating_sub(1);
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
                    (usize::from(s.bytes.is_some()), s.loans)
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
    fn byte_buffer_clone_is_inert_and_loans_block_mutation_and_freeze() {
        let domain = ValueOrigin::fresh().unwrap();
        let other = ValueOrigin::fresh().unwrap();
        let owner = ByteBuffer::empty(domain)
            .push(255, &ExecutionControl::uncancelled(), &mut |_| Ok(()))
            .unwrap();
        let clone = owner.clone();
        assert_eq!(clone, owner);
        assert_eq!(format!("{clone:?}"), "ByteBuffer(<opaque token>)");
        assert!(clone.borrow().is_err());
        assert!(clone.validate(domain, false).is_err());
        assert!(owner.validate(other, true).is_err());
        let loan = owner.borrow().unwrap();
        let nested = loan.borrow().unwrap();
        assert_eq!(loan.get(0).unwrap(), 255);
        assert!(loan.validate(domain, true).is_err());
        assert!(owner.validate(domain, true).is_err());
        drop(nested);
        drop(loan);
        owner.validate(domain, true).unwrap();
        let pointer = owner.lock().bytes.as_ref().unwrap().as_ptr();
        let bytes = owner.freeze().unwrap();
        assert_eq!(pointer, bytes.as_ptr());
        assert_eq!(&*bytes, &[255]);
        assert!(clone.validate(domain, false).is_err());
    }
    #[test]
    fn byte_buffer_failed_growth_and_cancellation_release_owner() {
        let domain = ValueOrigin::fresh().unwrap();
        let owner = ByteBuffer::empty(domain);
        let marker = owner.clone();
        let result = owner.push(1, &ExecutionControl::uncancelled(), &mut |_| {
            Err(ExecutionError::resource("test_quota", "refused"))
        });
        assert!(result.is_err());
        assert!(marker.lock().bytes.is_none());
        let owner = ByteBuffer::empty(domain);
        let marker = owner.clone();
        let control = ExecutionControl::uncancelled();
        let result = owner.push(2, &control, &mut |_| {
            control.cancel();
            Ok(())
        });
        assert!(result.is_err());
        assert!(marker.lock().bytes.is_none());
        let owner = ByteBuffer::empty(domain);
        let loan = owner.borrow().unwrap();
        drop(owner);
        assert!(loan.get(0).is_err());
    }
}
