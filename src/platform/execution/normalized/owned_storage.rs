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
    admitted_program: Option<ValueOrigin>,
    admission_valid: bool,
    // Physical absence of view carriers, independently of typed admission. Guarded
    // attachment preserves this fact; an arbitrary mutable callback invalidates it.
    view_free: bool,
}
pub struct OwnedStorage {
    // Read access can enter a child invocation without moving backing custody.
    origin: ValueOrigin,
    backing_origin: ValueOrigin,
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
            backing_origin: self.backing_origin,
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
        reject_view_fields(&fields, control)?;
        reserve(Self::ALLOCATION_BYTES)?;
        control.check()?;
        let product = Self {
            origin,
            backing_origin: origin,
            ty,
            storage: Arc::new(Mutex::new(Storage {
                fields: Some(fields),
                loans: 0,
                admitted_program: None,
                admission_valid: false,
                view_free: true,
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
        // A callback may retain an inert observation of this very allocation.
        // Refuse it before acquiring a mutex the callback already holds.
        if self.mode == Mode::Inert {
            return Err(reject());
        }
        let storage = self.lock();
        self.validate_storage(&storage, origin, consume)
    }
    fn validate_storage(
        &self,
        storage: &Storage,
        origin: ValueOrigin,
        consume: bool,
    ) -> Result<(), ExecutionError> {
        if self.origin != origin
            || self.mode == Mode::Inert
            || (self.mode == Mode::Owner && self.origin != self.backing_origin)
            || storage.fields.is_none()
            || (consume && (self.mode != Mode::Owner || storage.loans != 0))
        {
            return Err(reject());
        }
        Ok(())
    }
    /// The checked constructor admits the exact type and children before minting
    /// this allocation-bound certificate. Raw storage never starts certified.
    pub(super) fn establish_admission(&self, program: ValueOrigin) -> Result<(), ExecutionError> {
        if self.mode == Mode::Inert {
            return Err(reject());
        }
        let mut storage = self.lock();
        self.validate_storage(&storage, self.origin, true)?;
        restore_view_free(&mut storage, None)?;
        if storage
            .admitted_program
            .is_some_and(|existing| existing != program)
        {
            return Err(reject());
        }
        storage.admitted_program = Some(program);
        storage.admission_valid = true;
        Ok(())
    }
    pub(super) fn validate_admission(&self, program: ValueOrigin) -> Result<(), ExecutionError> {
        if self.mode == Mode::Inert {
            return Err(reject());
        }
        let storage = self.lock();
        self.validate_storage(&storage, self.origin, false)?;
        if storage.admitted_program != Some(program) || !storage.admission_valid {
            return Err(reject());
        }
        Ok(())
    }
    /// Return only a live allocation's currently valid prepared-program proof.
    pub(super) fn admitted_program(&self) -> Option<ValueOrigin> {
        if self.mode == Mode::Inert {
            return None;
        }
        let storage = self.lock();
        self.validate_storage(&storage, self.origin, false).ok()?;
        storage.admitted_program.filter(|_| storage.admission_valid)
    }
    pub(super) fn inherit_admission(&self, program: ValueOrigin) -> Result<(), ExecutionError> {
        if self.mode != Mode::Read {
            return Err(reject());
        }
        let mut storage = self.lock();
        self.validate_storage(&storage, self.origin, false)?;
        restore_view_free(&mut storage, None)?;
        if storage
            .admitted_program
            .is_some_and(|existing| existing != program)
        {
            return Err(reject());
        }
        storage.admitted_program = Some(program);
        storage.admission_valid = true;
        Ok(())
    }
    pub(super) fn adopt_scoped_read(
        &mut self,
        source: ValueOrigin,
        destination: ValueOrigin,
    ) -> Result<(), ExecutionError> {
        self.validate(source, false)?;
        if self.mode != Mode::Read {
            return Err(reject());
        }
        self.origin = destination;
        Ok(())
    }
    pub(super) fn borrow(&self) -> Result<Self, ExecutionError> {
        if self.mode == Mode::Inert {
            return Err(reject());
        }
        let mut storage = self.lock();
        self.validate_storage(&storage, self.origin, false)?;
        restore_view_free(&mut storage, None)?;
        storage.loans = storage.loans.checked_add(1).ok_or_else(reject)?;
        Ok(Self {
            origin: self.origin,
            backing_origin: self.backing_origin,
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
        StorageEscapeCheck::new(control).value(&value, 0)?;
        let mut storage = self.lock();
        self.validate_storage(&storage, origin, true)?;
        restore_view_free(&mut storage, Some(control))?;
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
        // The raw insertion API cannot preserve a checked-child certificate.
        // Its checked caller must re-establish admission after attaching the child.
        storage.admission_valid = false;
        let fields = storage.fields.as_mut().ok_or_else(reject)?;
        fields.push(value);
        Ok(())
    }
    /// Capacity belongs to the envelope and survives removal of its last item.
    pub(super) fn pop(
        &self,
        origin: ValueOrigin,
        control: &ExecutionControl,
    ) -> Result<Option<NormalizedValue>, ExecutionError> {
        if self.mode == Mode::Inert {
            return Err(reject());
        }
        let mut storage = self.lock();
        self.validate_storage(&storage, origin, true)?;
        control.check()?;
        restore_view_free(&mut storage, Some(control))?;
        let value = storage.fields.as_mut().map(Vec::pop).ok_or_else(reject)?;
        if value.is_some() {
            // Removal preserves a typed sequence only through its checked owner.
            // The same raw storage API can also receive a fixed product/choice.
            storage.admission_valid = false;
        }
        Ok(value)
    }
    /// Replacing one child neither grows nor copies the vector. As with raw
    /// insertion, only the checked caller may restore the parent certificate.
    pub(super) fn replace(
        &self,
        origin: ValueOrigin,
        index: usize,
        value: NormalizedValue,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        StorageEscapeCheck::new(control).value(&value, 0)?;
        if self.mode == Mode::Inert {
            return Err(reject());
        }
        let mut storage = self.lock();
        self.validate_storage(&storage, origin, true)?;
        control.check()?;
        restore_view_free(&mut storage, Some(control))?;
        let slot = storage
            .fields
            .as_mut()
            .and_then(|fields| fields.get_mut(index))
            .ok_or_else(reject)?;
        let displaced = std::mem::replace(slot, value);
        storage.admission_valid = false;
        Ok(displaced)
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
            if !storage.view_free {
                StorageEscapeCheck::new(control).value(value, 0)?;
            }
            if value.memory_form().is_some() {
                value.memory_validate(self.backing_origin, false)?;
                let mut borrowed = value.memory_borrow()?;
                let admission = storage.admitted_program.filter(|_| storage.admission_valid);
                match &mut borrowed {
                    NormalizedValue::ByteBuffer(token) => {
                        if let Some(program) = admission {
                            token.inherit_admission(program)?;
                        }
                        token.adopt_scoped_read(self.backing_origin, origin)?;
                    }
                    NormalizedValue::OwnedI64Cell(token) => {
                        if let Some(program) = admission {
                            token.inherit_admission(program)?;
                        }
                        token.adopt_scoped_read(self.backing_origin, origin)?;
                    }
                    NormalizedValue::OwnedProduct(token) => {
                        if let Some(program) = admission {
                            token.inherit_admission(program)?;
                        }
                        token.adopt_scoped_read(self.backing_origin, origin)?;
                    }
                    NormalizedValue::OwnedChoice(token) => {
                        if let Some(program) = admission {
                            token.inherit_admission(program)?;
                        }
                        token.adopt_scoped_read(self.backing_origin, origin)?;
                    }
                    NormalizedValue::OwnedSequence(token) => {
                        if let Some(program) = admission {
                            token.inherit_admission(program)?;
                        }
                        token.adopt_scoped_read(self.backing_origin, origin)?;
                    }
                    _ => return Err(reject()),
                }
                return Ok(borrowed);
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
        let fields = storage.fields.as_deref().ok_or_else(reject)?;
        if !storage.view_free {
            reject_view_fields_uncontrolled(fields)?;
        }
        inspect(fields)
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
            // Interrupted descendant adoption is cleanup custody, never a new
            // reusable typed root. Retain the program binding across invalidation.
            let was_admitted = storage.admission_valid;
            restore_view_free(&mut storage, None)?;
            storage.admission_valid = false;
            storage.view_free = false;
            let fields = storage.fields.as_deref_mut().ok_or_else(reject)?;
            adopt(fields)?;
            // A raw callback cannot smuggle a new capability into an Owned envelope.
            // Failed callbacks/admission keep cleanup custody and no certificate.
            reject_view_fields_uncontrolled(fields)?;
            storage.view_free = true;
            storage.admission_valid = was_admitted;
        }
        self.origin = destination;
        self.backing_origin = destination;
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
        if !storage.view_free {
            StorageEscapeCheck::new(control).value(value, 0)?;
        }
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
                NormalizedValue::ReadView(_)
                | NormalizedValue::ByteBuffer(_)
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
        let fields = {
            let mut storage = self.lock();
            restore_view_free(&mut storage, Some(control))?;
            storage.fields.take().ok_or_else(reject)?
        };
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

// A negative guard only: callers still independently admit closed Owned types,
// child category/provenance, depth and growth before establishing a certificate.
fn reject_view_fields(
    fields: &[NormalizedValue],
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    let mut guard = StorageEscapeCheck::new(control);
    for field in fields {
        guard.value(field, 0)?;
    }
    Ok(())
}

struct StorageEscapeCheck<'a> {
    control: Option<&'a ExecutionControl>,
    visits: u64,
}

impl<'a> StorageEscapeCheck<'a> {
    fn new(control: &'a ExecutionControl) -> Self {
        Self {
            control: Some(control),
            visits: 0,
        }
    }

    fn value(&mut self, value: &NormalizedValue, depth: usize) -> Result<(), ExecutionError> {
        if let Some(control) = self.control {
            control.check()?;
        }
        if depth > 256 {
            return Err(ExecutionError::resource(
                "normalized_value_depth",
                "owned storage escape inspection exceeds finite value depth 256",
            ));
        }
        self.visits = self
            .visits
            .checked_add(1)
            .filter(|visits| *visits <= super::value::MAXIMUM_ADMISSION_ITEMS + 1)
            .ok_or_else(|| {
                ExecutionError::resource(
                    "normalized_collection_items",
                    "owned storage escape inspection exceeds finite admission items",
                )
            })?;
        match value {
            NormalizedValue::ReadView(_) => {
                return Err(view_escape());
            }
            NormalizedValue::OwnedProduct(storage) => self.storage(storage, depth)?,
            NormalizedValue::OwnedChoice(choice) => self.storage(&choice.storage, depth)?,
            NormalizedValue::OwnedSequence(sequence) => self.storage(&sequence.storage, depth)?,
            NormalizedValue::Record(super::value::NormalizedRecord::Nominal { fields, .. }) => {
                for field in fields.iter() {
                    self.value(field, depth + 1)?;
                }
            }
            NormalizedValue::Record(super::value::NormalizedRecord::Structural { fields }) => {
                for (_, field) in fields.iter() {
                    self.value(field, depth + 1)?;
                }
            }
            NormalizedValue::List(values) => self.list(values, depth)?,
            NormalizedValue::Map(values) => self.map(values, depth)?,
            NormalizedValue::Variant { payload, .. } | NormalizedValue::Option(payload) => {
                if let Some(child) = payload {
                    self.value(child, depth + 1)?;
                }
            }
            NormalizedValue::Result { value, .. } => self.value(value, depth + 1)?,
            NormalizedValue::Function {
                read_scope_arguments,
                bound_arguments,
                ..
            } => {
                if !read_scope_arguments.is_empty() {
                    return Err(view_escape());
                }
                if let Some(arguments) = bound_arguments {
                    for argument in arguments.iter() {
                        self.value(argument, depth + 1)?;
                    }
                }
            }
            NormalizedValue::Unit
            | NormalizedValue::Bool(_)
            | NormalizedValue::I64(_)
            | NormalizedValue::F64(_)
            | NormalizedValue::Bytes(_)
            | NormalizedValue::Text(_)
            | NormalizedValue::StaticText(_)
            | NormalizedValue::ByteBuffer(_)
            | NormalizedValue::OwnedI64Cell(_)
            | NormalizedValue::Resource(_) => {}
        }
        Ok(())
    }

    fn storage(&mut self, token: &OwnedStorage, depth: usize) -> Result<(), ExecutionError> {
        // Inert observations cannot supply child storage. Reject before locking:
        // a raw self-observation inside a callback must never deadlock this walk.
        if token.mode == Mode::Inert {
            return Err(reject());
        }
        let storage = token.lock();
        if storage.view_free {
            return Ok(());
        }
        for field in storage.fields.as_deref().ok_or_else(reject)? {
            self.value(field, depth + 1)?;
        }
        Ok(())
    }

    #[inline(never)]
    fn list(&mut self, values: &super::list::List, depth: usize) -> Result<(), ExecutionError> {
        let mut cursor = values.iter();
        for child in &mut cursor {
            self.value(child, depth + 1)?;
        }
        Ok(())
    }

    #[inline(never)]
    fn map(&mut self, values: &super::map::Map, depth: usize) -> Result<(), ExecutionError> {
        let mut cursor = values.iter();
        for (_, child) in &mut cursor {
            self.value(child, depth + 1)?;
        }
        Ok(())
    }
}

fn reject_view_fields_uncontrolled(fields: &[NormalizedValue]) -> Result<(), ExecutionError> {
    // These existing callback APIs carry no execution control. Do not allocate a
    // substitute control or silently grant them a new cancellation domain.
    let mut guard = StorageEscapeCheck {
        control: None,
        visits: 0,
    };
    for field in fields {
        guard.value(field, 0)?;
    }
    Ok(())
}

fn restore_view_free(
    storage: &mut Storage,
    control: Option<&ExecutionControl>,
) -> Result<(), ExecutionError> {
    if !storage.view_free {
        let mut guard = StorageEscapeCheck { control, visits: 0 };
        for field in storage.fields.as_deref().ok_or_else(reject)? {
            guard.value(field, 0)?;
        }
        storage.view_free = true;
    }
    Ok(())
}

fn view_escape() -> ExecutionError {
    ExecutionError::new(
        crate::platform::execution::ExecutionFailureClass::Trap,
        "normalized_read_view_escape",
        "read-view values, scopes, or nested types cannot cross an ordinary value boundary",
    )
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

#[cfg(test)]
mod scoped_read_tests {
    use super::super::owned_choice::OwnedChoice;
    use super::super::owned_i64_cell::{OwnedI64Cell, StorageObservation as Cells};
    use super::super::owned_sequence::OwnedSequence;
    use super::*;

    #[test]
    fn scoped_root_read_visits_only_the_selected_child() {
        let cells = Cells::start();
        let source = ValueOrigin::fresh().unwrap();
        let destination = ValueOrigin::fresh().unwrap();
        let program = ValueOrigin::fresh().unwrap();
        let ty = TypeObjectDigest::from_bytes([201; 32]);
        let control = ExecutionControl::uncancelled();
        for length in [1, 4096] {
            let fields = (0..length)
                .map(|index| NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(source, index as i64)))
                .collect();
            let mut owner =
                OwnedStorage::create(source, ty, fields, &control, &mut |_| Ok(())).unwrap();
            assert!(owner.validate_admission(program).is_err());
            owner.establish_admission(program).unwrap();
            let vector = owner.vector_identity();
            assert!(owner.adopt_scoped_read(source, destination).is_err());
            let mut read = owner.borrow().unwrap();
            read.adopt_scoped_read(source, destination).unwrap();
            assert_eq!(read.backing_origin, source);
            assert_eq!(read.vector_identity(), vector);
            assert_eq!(cells.live(), (length, 0));
            let NormalizedValue::OwnedI64Cell(selected) = read
                .borrow_field(destination, length - 1, &control, &mut |_| {
                    panic!("read capture and owned projection allocate no payload")
                })
                .unwrap()
            else {
                panic!("selected cell")
            };
            selected.validate(destination, false).unwrap();
            selected.validate_admission(program).unwrap();
            assert_eq!(selected.read().unwrap(), (length - 1) as i64);
            assert_eq!(cells.live(), (length, 1));
            assert!(owner.validate(source, true).is_err());
            drop(selected);
            drop(read);
            owner
                .inspect_transfer(source, |fields| {
                    for (index, value) in fields.iter().enumerate() {
                        value.memory_validate(source, true)?;
                        let NormalizedValue::OwnedI64Cell(cell) = value else {
                            panic!("cell owner")
                        };
                        assert_eq!(
                            cell.validate_admission(program).is_ok(),
                            index == length - 1,
                            "only the projected child inherits admission"
                        );
                    }
                    Ok(())
                })
                .unwrap();
            assert_eq!(owner.vector_identity(), vector);
            drop(owner);
            assert_eq!(cells.live(), (0, 0));
        }
    }

    #[test]
    fn scoped_nested_projection_reborrows_with_original_backing_custody() {
        let source = ValueOrigin::fresh().unwrap();
        let destination = ValueOrigin::fresh().unwrap();
        let grandchild = ValueOrigin::fresh().unwrap();
        let program = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let ty = TypeObjectDigest::from_bytes([202; 32]);
        let cell = OwnedI64Cell::new(source, 97);
        let identity = cell.allocation_identity();
        let sequence = OwnedSequence::create(source, ty, &control, &mut |_| Ok(()))
            .unwrap()
            .push(
                source,
                NormalizedValue::OwnedI64Cell(cell),
                &control,
                &mut |_| Ok(()),
            )
            .unwrap();
        let choice = OwnedChoice::create(
            source,
            ty,
            0,
            NormalizedValue::OwnedSequence(sequence),
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        let owner = OwnedStorage::create(
            source,
            ty,
            vec![NormalizedValue::OwnedChoice(choice)],
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        owner.establish_admission(program).unwrap();
        let mut root = owner.borrow().unwrap();
        root.adopt_scoped_read(source, destination).unwrap();
        let NormalizedValue::OwnedChoice(choice) = root
            .borrow_field(destination, 0, &control, &mut |_| Ok(()))
            .unwrap()
        else {
            panic!("choice read")
        };
        choice.validate_admission(program).unwrap();
        let NormalizedValue::OwnedSequence(sequence) = choice
            .borrow_payload(destination, &control, &mut |_| Ok(()))
            .unwrap()
        else {
            panic!("sequence read")
        };
        sequence.validate_admission(program).unwrap();
        let mut nested = sequence.borrow().unwrap();
        nested.adopt_scoped_read(destination, grandchild).unwrap();
        let NormalizedValue::OwnedI64Cell(cell) = nested
            .borrow_item(grandchild, 0, &control, &mut |_| Ok(()))
            .unwrap()
        else {
            panic!("cell read")
        };
        cell.validate(grandchild, false).unwrap();
        cell.validate_admission(program).unwrap();
        assert_eq!(cell.allocation_identity(), identity);
        assert_eq!(cell.read().unwrap(), 97);
        assert!(owner.validate(source, true).is_err());
        drop(cell);
        drop(nested);
        drop(sequence);
        drop(choice);
        drop(root);
        owner.validate(source, true).unwrap();
        owner.validate_admission(program).unwrap();
    }

    #[test]
    fn admission_remains_program_bound_after_mutation_and_failed_adoption() {
        let source = ValueOrigin::fresh().unwrap();
        let destination = ValueOrigin::fresh().unwrap();
        let program = ValueOrigin::fresh().unwrap();
        let foreign = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let ty = TypeObjectDigest::from_bytes([203; 32]);
        let mut owner =
            OwnedStorage::create(source, ty, vec![], &control, &mut |_| Ok(())).unwrap();
        owner.establish_admission(program).unwrap();
        assert!(owner.clone().validate_admission(program).is_err());
        owner
            .push(
                source,
                NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(source, 11)),
                &control,
                &mut |_| Ok(()),
            )
            .unwrap();
        assert!(owner.validate_admission(program).is_err());
        assert!(owner.establish_admission(foreign).is_err());
        owner.establish_admission(program).unwrap();
        let read = owner.borrow().unwrap();
        assert!(read.establish_admission(program).is_err());
        assert!(read.inherit_admission(foreign).is_err());
        drop(read);
        assert!(
            owner
                .adopt_transfer(source, destination, |fields| {
                    let [NormalizedValue::OwnedI64Cell(cell)] = fields else {
                        panic!("one cell")
                    };
                    cell.adopt_transfer(source, destination)?;
                    Err(ExecutionError::resource("test_adoption", "interrupted"))
                })
                .is_err()
        );
        assert_eq!(owner.backing_origin, source);
        assert!(owner.validate_admission(program).is_err());
        assert!(owner.establish_admission(foreign).is_err());
    }

    #[test]
    fn raw_removal_invalidates_a_certified_fixed_product_shape() {
        let origin = ValueOrigin::fresh().unwrap();
        let program = ValueOrigin::fresh().unwrap();
        let foreign = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let ty = TypeObjectDigest::from_bytes([204; 32]);
        let owner = OwnedStorage::create(
            origin,
            ty,
            vec![NormalizedValue::I64(31)],
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        owner.establish_admission(program).unwrap();
        assert_eq!(owner.admitted_program(), Some(program));
        assert_eq!(
            owner.pop(origin, &control).unwrap(),
            Some(NormalizedValue::I64(31))
        );
        assert_eq!(owner.admitted_program(), None);
        assert!(owner.validate_admission(program).is_err());
        assert!(owner.establish_admission(foreign).is_err());

        let empty = OwnedStorage::create(origin, ty, vec![], &control, &mut |_| Ok(())).unwrap();
        empty.establish_admission(program).unwrap();
        assert_eq!(empty.pop(origin, &control).unwrap(), None);
        assert_eq!(empty.admitted_program(), Some(program));
    }
}
