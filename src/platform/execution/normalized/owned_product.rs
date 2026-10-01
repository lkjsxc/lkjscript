//! A single owner of a fixed field vector. Shared control never shares ownership.
use super::value::{NormalizedValue, ValueOrigin};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{TypeObjectDigest, contract::MAXIMUM_TYPE_DEPTH};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    Owner,
    Read,
    Inert,
}
struct Storage {
    fields: Option<Vec<NormalizedValue>>,
    loans: usize,
}
pub struct OwnedProduct {
    origin: ValueOrigin,
    ty: TypeObjectDigest,
    storage: Arc<Mutex<Storage>>,
    mode: Mode,
}
fn reject() -> ExecutionError {
    ExecutionError::resource(
        "normalized_product_token",
        "owned product token is stale, foreign, borrowed, or unadmitted",
    )
}
impl Clone for OwnedProduct {
    fn clone(&self) -> Self {
        Self {
            origin: self.origin,
            ty: self.ty,
            storage: Arc::clone(&self.storage),
            mode: Mode::Inert,
        }
    }
}
impl PartialEq for OwnedProduct {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.ty == other.ty
            && Arc::ptr_eq(&self.storage, &other.storage)
    }
}
impl Eq for OwnedProduct {}
impl std::fmt::Debug for OwnedProduct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OwnedProduct(<opaque token>)")
    }
}
impl OwnedProduct {
    pub(super) const ALLOCATION_BYTES: u64 = (std::mem::size_of::<Self>()
        + std::mem::size_of::<Mutex<Storage>>()
        + 2 * std::mem::size_of::<usize>()) as u64;
    fn lock(&self) -> MutexGuard<'_, Storage> {
        self.storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    // Callers admit exact closed type/children and reserve field-vector storage before growth.
    pub(super) fn create(
        origin: ValueOrigin,
        ty: TypeObjectDigest,
        fields: Vec<NormalizedValue>,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        reserve(Self::ALLOCATION_BYTES)?;
        control.check()?;
        let product = Self {
            origin,
            ty,
            storage: Arc::new(Mutex::new(Storage {
                fields: Some(fields),
                loans: 0,
            })),
            mode: Mode::Owner,
        };
        #[cfg(test)]
        OBSERVED.with(|entries| {
            if let Some(entries) = entries.borrow_mut().as_mut() {
                entries.push(Observed {
                    storage: Arc::downgrade(&product.storage),
                    packed: allocation_identities(product.lock().fields.as_ref().unwrap()),
                    unpacked: None,
                });
            }
        });
        Ok(product)
    }
    pub(super) fn ty(&self) -> TypeObjectDigest {
        self.ty
    }
    pub(super) fn validate(
        &self,
        origin: ValueOrigin,
        consume: bool,
    ) -> Result<(), ExecutionError> {
        let storage = self.lock();
        if self.origin != origin
            || self.mode == Mode::Inert
            || storage.fields.is_none()
            || (consume && (self.mode != Mode::Owner || storage.loans != 0))
        {
            return Err(reject());
        }
        Ok(())
    }
    pub(super) fn borrow(&self) -> Result<Self, ExecutionError> {
        self.validate(self.origin, false)?;
        let mut storage = self.lock();
        storage.loans = storage.loans.checked_add(1).ok_or_else(reject)?;
        Ok(Self {
            origin: self.origin,
            ty: self.ty,
            storage: Arc::clone(&self.storage),
            mode: Mode::Read,
        })
    }
    pub(super) fn is_borrowed(&self) -> bool {
        self.mode == Mode::Read
    }
    pub(super) fn owns_live_loans(&self) -> bool {
        self.mode == Mode::Owner && self.lock().loans != 0
    }
    /// Callers resolve the index from this token's exact type and independently
    /// admit the returned closed ordinary value. No child ownership is transferred.
    pub(super) fn read_metadata(
        &self,
        origin: ValueOrigin,
        index: usize,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.validate(origin, false)?;
        if self.mode != Mode::Read {
            return Err(reject());
        }
        control.check()?;
        let storage = self.lock();
        let value = storage
            .fields
            .as_ref()
            .and_then(|fields| fields.get(index))
            .ok_or_else(reject)?;
        // Records, lists, maps, text and bytes share immutable backing. Only the
        // inline sum/option spine allocates during an ordinary value clone.
        let mut current = value;
        let mut bytes = 0_u64;
        loop {
            control.check()?;
            current = match current {
                NormalizedValue::Variant {
                    payload: Some(child),
                    ..
                }
                | NormalizedValue::Option(Some(child))
                | NormalizedValue::Result { value: child, .. } => {
                    bytes = bytes
                        .checked_add(std::mem::size_of::<NormalizedValue>() as u64)
                        .ok_or_else(reject)?;
                    child
                }
                NormalizedValue::ByteBuffer(_)
                | NormalizedValue::OwnedI64Cell(_)
                | NormalizedValue::OwnedProduct(_)
                | NormalizedValue::Resource(_)
                | NormalizedValue::Function { .. } => return Err(reject()),
                _ => break,
            };
        }
        if bytes != 0 {
            reserve(bytes)?;
        }
        control.check()?;
        Ok(value.clone())
    }

    pub(super) fn unpack(
        self,
        origin: ValueOrigin,
        ty: TypeObjectDigest,
        control: &ExecutionControl,
    ) -> Result<Vec<NormalizedValue>, ExecutionError> {
        self.validate(origin, true)?;
        if self.ty != ty {
            return Err(reject());
        }
        control.check()?;
        let fields = self.lock().fields.take().ok_or_else(reject)?;
        #[cfg(test)]
        OBSERVED.with(|entries| {
            if let Some(entries) = entries.borrow_mut().as_mut()
                && let Some(entry) = entries
                    .iter_mut()
                    .find(|entry| entry.storage.ptr_eq(&Arc::downgrade(&self.storage)))
            {
                entry.unpacked = Some(allocation_identities(&fields));
            }
        });
        Ok(fields)
    }
    fn revoke(&mut self) -> Option<Vec<NormalizedValue>> {
        if self.mode != Mode::Owner {
            return None;
        }
        self.mode = Mode::Inert;
        self.lock().fields.take()
    }
}

#[cfg(test)]
thread_local! {static OBSERVED:std::cell::RefCell<Option<Vec<Observed>>> = const { std::cell::RefCell::new(None) };}
#[cfg(test)]
struct Observed {
    storage: std::sync::Weak<Mutex<Storage>>,
    packed: Vec<usize>,
    unpacked: Option<Vec<usize>>,
}
#[cfg(test)]
fn allocation_identities(fields: &[NormalizedValue]) -> Vec<usize> {
    fields
        .iter()
        .map(|field| match field {
            NormalizedValue::ByteBuffer(buffer) => buffer.allocation_identity(),
            NormalizedValue::OwnedI64Cell(cell) => cell.allocation_identity(),
            NormalizedValue::OwnedProduct(product) => Arc::as_ptr(&product.storage) as usize,
            _ => 0,
        })
        .collect()
}
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
    pub(super) fn created(&self) -> usize {
        OBSERVED.with(|entries| entries.borrow().as_ref().unwrap().len())
    }
    pub(super) fn assert_transfers_preserve_allocations(&self) {
        OBSERVED.with(|entries| {
            let entries = entries.borrow();
            let mut transfers = 0;
            for entry in entries.as_ref().unwrap() {
                if let Some(unpacked) = &entry.unpacked {
                    assert_eq!(&entry.packed, unpacked);
                    transfers += 1;
                }
            }
            assert!(transfers > 0);
        });
    }
    pub(super) fn live(&self) -> (usize, usize) {
        OBSERVED.with(|entries| {
            entries
                .borrow()
                .as_ref()
                .unwrap()
                .iter()
                .filter_map(|entry| entry.storage.upgrade())
                .map(|s| {
                    let s = s.lock().unwrap();
                    (usize::from(s.fields.is_some()), s.loans)
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
impl Drop for OwnedProduct {
    fn drop(&mut self) {
        if self.mode == Mode::Read {
            let mut storage = self.lock();
            storage.loans = storage.loans.saturating_sub(1);
            return;
        }
        let Some(fields) = self.revoke() else {
            return;
        };
        // Finite admitted type depth bounds this allocation-free cleanup stack. Nested
        // owners are revoked before their inert shells drop, avoiding recursive Drop.
        let mut stack: [Option<std::vec::IntoIter<NormalizedValue>>; MAXIMUM_TYPE_DEPTH + 1] =
            std::array::from_fn(|_| None);
        stack[0] = Some(fields.into_iter());
        let mut depth = 0;
        loop {
            let value = stack[depth].as_mut().and_then(Iterator::next);
            match value {
                Some(NormalizedValue::OwnedProduct(mut product)) => {
                    if let Some(fields) = product.revoke() {
                        depth += 1;
                        stack[depth] = Some(fields.into_iter());
                    }
                }
                Some(value) => drop(value),
                None => {
                    stack[depth] = None;
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
            }
        }
    }
}
