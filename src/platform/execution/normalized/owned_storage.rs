//! Shared custody of an owned child vector. Sharing control never shares ownership.
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
pub struct OwnedStorage {
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
impl Clone for OwnedStorage {
    fn clone(&self) -> Self {
        Self {
            origin: self.origin,
            ty: self.ty,
            storage: Arc::clone(&self.storage),
            mode: Mode::Inert,
        }
    }
}
impl PartialEq for OwnedStorage {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.ty == other.ty
            && Arc::ptr_eq(&self.storage, &other.storage)
    }
}
impl Eq for OwnedStorage {}
impl std::fmt::Debug for OwnedStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OwnedStorage(<opaque token>)")
    }
}
impl OwnedStorage {
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
                    revoked_with_loans: None,
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
    #[cfg(test)]
    pub(super) fn vector_identity(&self) -> usize {
        self.lock().fields.as_ref().unwrap().as_ptr() as usize
    }
    #[cfg(test)]
    pub(super) fn vector_capacity(&self) -> usize {
        self.lock().fields.as_ref().unwrap().capacity()
    }
    pub(super) fn len(&self, origin: ValueOrigin) -> Result<usize, ExecutionError> {
        self.validate(origin, false)?;
        self.lock().fields.as_ref().map(Vec::len).ok_or_else(reject)
    }
    /// Admit dynamic storage growth before reallocating or attaching the new
    /// child. The child remains in the caller's custody on every failure.
    pub(super) fn push(
        &self,
        origin: ValueOrigin,
        value: NormalizedValue,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        self.validate(origin, true)?;
        control.check()?;
        let mut storage = self.lock();
        let fields = storage.fields.as_mut().ok_or_else(reject)?;
        let length = fields.len().checked_add(1).ok_or_else(reject)?;
        if length as u64 > super::value::MAXIMUM_ADMISSION_ITEMS {
            return Err(ExecutionError::resource(
                "normalized_sequence_storage",
                "owned sequence exceeds its finite item bound",
            ));
        }
        if length > fields.capacity() {
            let capacity = fields
                .capacity()
                .saturating_mul(2)
                .max(8)
                .max(length)
                .min(super::value::MAXIMUM_ADMISSION_ITEMS as usize);
            let bytes = super::value::collection_storage_bytes(
                capacity as u64,
                std::mem::size_of::<NormalizedValue>() as u64,
                "normalized_sequence_storage",
            )?;
            if bytes > super::value::MAXIMUM_VALUE_ALLOCATION_BYTES {
                return Err(ExecutionError::resource(
                    "normalized_sequence_storage",
                    "owned sequence exceeds its finite storage bound",
                ));
            }
            reserve(bytes)?;
            control.check()?;
            fields
                .try_reserve_exact(capacity - fields.len())
                .map_err(|_| {
                    ExecutionError::resource(
                        "normalized_sequence_storage",
                        "cannot allocate admitted owned sequence storage",
                    )
                })?;
        }
        control.check()?;
        fields.push(value);
        Ok(())
    }
    /// Capacity belongs to the envelope and survives removal of its last item.
    pub(super) fn pop(
        &self,
        origin: ValueOrigin,
        control: &ExecutionControl,
    ) -> Result<Option<NormalizedValue>, ExecutionError> {
        self.validate(origin, true)?;
        control.check()?;
        self.lock().fields.as_mut().map(Vec::pop).ok_or_else(reject)
    }
    /// Project a read token while a separately retained parent loan keeps the
    /// complete product in custody. The storage lock never crosses evaluation
    /// of the caller's lexical body.
    pub(super) fn borrow_field(
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
        {
            let storage = self.lock();
            let value = storage
                .fields
                .as_ref()
                .and_then(|fields| fields.get(index))
                .ok_or_else(reject)?;
            if value.memory_form().is_some() {
                value.memory_validate(origin, false)?;
                return value.memory_borrow();
            }
        }
        self.read_metadata(origin, index, control, reserve)
    }
    /// Borrowed inspection cannot expose a child owner beyond the callback.
    pub(super) fn inspect_transfer<R>(
        &self,
        source: ValueOrigin,
        inspect: impl FnOnce(&[NormalizedValue]) -> Result<R, ExecutionError>,
    ) -> Result<R, ExecutionError> {
        if self.origin != source || self.mode != Mode::Owner {
            return Err(reject());
        }
        self.validate(source, true)?;
        let storage = self.lock();
        inspect(storage.fields.as_deref().ok_or_else(reject)?)
    }
    /// The consuming envelope keeps custody if cancellation interrupts children.
    /// Destruction does not depend on their possibly mixed intermediate domains.
    pub(super) fn adopt_transfer(
        &mut self,
        source: ValueOrigin,
        destination: ValueOrigin,
        adopt: impl FnOnce(&mut [NormalizedValue]) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        if self.origin != source || self.mode != Mode::Owner {
            return Err(reject());
        }
        self.validate(source, true)?;
        {
            let mut storage = self.lock();
            adopt(storage.fields.as_deref_mut().ok_or_else(reject)?)?;
        }
        self.origin = destination;
        Ok(())
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
                | NormalizedValue::OwnedChoice(_)
                | NormalizedValue::OwnedSequence(_)
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
    pub(super) fn revoke(&mut self) -> Option<Vec<NormalizedValue>> {
        if self.mode != Mode::Owner {
            return None;
        }
        self.mode = Mode::Inert;
        let mut storage = self.lock();
        #[cfg(test)]
        OBSERVED.with(|entries| {
            if let Some(entries) = entries.borrow_mut().as_mut()
                && let Some(entry) = entries
                    .iter_mut()
                    .find(|entry| entry.storage.ptr_eq(&Arc::downgrade(&self.storage)))
            {
                entry.revoked_with_loans = Some(storage.loans);
            }
        });
        storage.fields.take()
    }
}

#[cfg(test)]
thread_local! {static OBSERVED:std::cell::RefCell<Option<Vec<Observed>>> = const { std::cell::RefCell::new(None) };}
#[cfg(test)]
struct Observed {
    storage: std::sync::Weak<Mutex<Storage>>,
    packed: Vec<usize>,
    unpacked: Option<Vec<usize>>,
    revoked_with_loans: Option<usize>,
}
#[cfg(test)]
fn allocation_identities(fields: &[NormalizedValue]) -> Vec<usize> {
    fields
        .iter()
        .map(|field| match field {
            NormalizedValue::ByteBuffer(buffer) => buffer.allocation_identity(),
            NormalizedValue::OwnedI64Cell(cell) => cell.allocation_identity(),
            NormalizedValue::OwnedProduct(product) => Arc::as_ptr(&product.storage) as usize,
            NormalizedValue::OwnedChoice(choice) => Arc::as_ptr(&choice.storage.storage) as usize,
            NormalizedValue::OwnedSequence(sequence) => {
                Arc::as_ptr(&sequence.storage.storage) as usize
            }
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
    pub(super) fn assert_owners_released_after_loans(&self) {
        OBSERVED.with(|entries| {
            assert!(
                entries
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .iter()
                    .all(|entry| entry.revoked_with_loans.is_none_or(|loans| loans == 0))
            );
        });
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
impl Drop for OwnedStorage {
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
                Some(NormalizedValue::OwnedChoice(mut choice)) => {
                    if let Some(fields) = choice.storage.revoke() {
                        depth += 1;
                        stack[depth] = Some(fields.into_iter());
                    }
                }
                Some(NormalizedValue::OwnedSequence(mut sequence)) => {
                    if let Some(fields) = sequence.storage.revoke() {
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
