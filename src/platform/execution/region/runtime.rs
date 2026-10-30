//! Evaluator custody for lexical slabs. The arena owns buffers; tokens own no
//! allocation or reference count. Scoped workers borrow the same arena.

use super::{persistence, *};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

static NEXT_ARENA: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Identity {
    arena: u64,
    slot: usize,
    generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Pointer {
    identity: Identity,
    offset: u64,
}

impl Pointer {
    pub(crate) fn in_region(identity: Identity, offset: u64) -> Self {
        Self { identity, offset }
    }
    pub(crate) fn belongs_to(self, identity: Identity) -> bool {
        self.identity == identity
    }
    pub(crate) fn offset(self) -> u64 {
        self.offset
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Access {
    Unique,
    Frozen,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Capability<Domain> {
    identity: Identity,
    domain: Domain,
    access: Access,
    borrowed: bool,
    live: bool,
}

// Raw observations cannot duplicate custody. Evaluators must explicitly admit
// a read loan or transfer the original token.
impl<Domain: Copy> Clone for Capability<Domain> {
    fn clone(&self) -> Self {
        Self {
            live: false,
            ..*self
        }
    }
}

impl<Domain> Capability<Domain> {
    pub(crate) fn access(&self) -> Access {
        self.access
    }

    pub(crate) fn is_borrowed(&self) -> bool {
        self.borrowed
    }

    pub(crate) fn identity(&self) -> Identity {
        self.identity
    }
}

struct Entry<Domain> {
    slab: Slab,
    owner: Domain,
    access: Access,
}

struct Slot<Domain> {
    generation: u64,
    entry: Option<Entry<Domain>>,
    next_free: Option<usize>,
}

struct State<Domain> {
    slots: Vec<Slot<Domain>>,
    free: Option<usize>,
    live: usize,
}

pub(crate) struct Arena<Domain> {
    identity: u64,
    state: RwLock<State<Domain>>,
}

impl<Domain: Copy + Eq> Arena<Domain> {
    pub(crate) fn new() -> Result<Self, ExecutionError> {
        let identity = NEXT_ARENA
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| capacity_error())?;
        Ok(Self {
            identity,
            state: RwLock::new(State {
                slots: Vec::new(),
                free: None,
                live: 0,
            }),
        })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.read_state().live == 0
    }

    fn read_state(&self) -> RwLockReadGuard<'_, State<Domain>> {
        self.state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write_state(&self) -> RwLockWriteGuard<'_, State<Domain>> {
        self.state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn open(
        &self,
        domain: Domain,
        capacity: u64,
        control: &ExecutionControl,
        mut reserve: impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Capability<Domain>, ExecutionError> {
        control.check()?;
        let mut state = self.write_state();
        let live = state.live.checked_add(1).ok_or_else(capacity_error)?;
        let slot = if let Some(slot) = state.free {
            slot
        } else {
            if state.slots.len() == state.slots.capacity() {
                let next = slot_capacity::<Domain>(state.slots.len())?;
                let charge = next
                    .checked_mul(std::mem::size_of::<Slot<Domain>>())
                    .ok_or_else(capacity_error)?;
                reserve(u64::try_from(charge).map_err(|_| capacity_error())?)?;
                control.check()?;
                let mut slots = Vec::new();
                slots
                    .try_reserve_exact(next)
                    .map_err(|_| capacity_error())?;
                slots.append(&mut state.slots);
                state.slots = slots;
            }
            state.slots.len()
        };
        let generation = state
            .slots
            .get(slot)
            .map_or(Some(1), |slot| slot.generation.checked_add(1))
            .ok_or_else(capacity_error)?;
        let slab = with_region(capacity, control, &mut reserve, |region| Ok(region.slab))?;
        control.check()?;
        let entry = Entry {
            slab,
            owner: domain,
            access: Access::Unique,
        };
        if slot == state.slots.len() {
            state.slots.push(Slot {
                generation,
                entry: Some(entry),
                next_free: None,
            });
        } else {
            state.free = state.slots[slot].next_free;
            state.slots[slot] = Slot {
                generation,
                entry: Some(entry),
                next_free: None,
            };
        }
        state.live = live;
        Ok(Capability {
            identity: Identity {
                arena: self.identity,
                slot,
                generation,
            },
            domain,
            access: Access::Unique,
            borrowed: false,
            live: true,
        })
    }

    fn entry<'a>(
        &self,
        state: &'a State<Domain>,
        cap: &Capability<Domain>,
        domain: Domain,
        consume: bool,
    ) -> Result<&'a Entry<Domain>, ExecutionError> {
        if !cap.live
            || cap.domain != domain
            || cap.identity.arena != self.identity
            || (consume && cap.borrowed)
        {
            return Err(custody_error());
        }
        let slot = state
            .slots
            .get(cap.identity.slot)
            .filter(|slot| slot.generation == cap.identity.generation)
            .ok_or_else(custody_error)?;
        let entry = slot.entry.as_ref().ok_or_else(custody_error)?;
        if entry.access != cap.access
            || (!cap.borrowed && entry.owner != domain)
            || (cap.borrowed && entry.access != Access::Frozen && entry.owner != domain)
        {
            return Err(custody_error());
        }
        Ok(entry)
    }

    pub(crate) fn validate(
        &self,
        cap: &Capability<Domain>,
        domain: Domain,
        consume: bool,
    ) -> Result<(), ExecutionError> {
        self.entry(&self.read_state(), cap, domain, consume)
            .map(|_| ())
    }

    pub(crate) fn borrow(
        &self,
        cap: &Capability<Domain>,
        source: Domain,
        destination: Domain,
    ) -> Result<Capability<Domain>, ExecutionError> {
        let state = self.read_state();
        self.entry(&state, cap, source, false)?;
        if cap.access != Access::Frozen && source != destination {
            return Err(custody_error());
        }
        Ok(Capability {
            domain: destination,
            borrowed: true,
            live: true,
            ..*cap
        })
    }

    pub(crate) fn transfer(
        &self,
        mut cap: Capability<Domain>,
        source: Domain,
        destination: Domain,
    ) -> Result<Capability<Domain>, ExecutionError> {
        let mut state = self.write_state();
        self.entry(&state, &cap, source, true)?;
        state.slots[cap.identity.slot]
            .entry
            .as_mut()
            .ok_or_else(custody_error)?
            .owner = destination;
        cap.domain = destination;
        Ok(cap)
    }

    pub(crate) fn freeze(
        &self,
        mut cap: Capability<Domain>,
        domain: Domain,
        control: &ExecutionControl,
    ) -> Result<Capability<Domain>, ExecutionError> {
        control.check()?;
        let mut state = self.write_state();
        self.entry(&state, &cap, domain, true)?;
        if cap.access != Access::Unique {
            return Err(custody_error());
        }
        state.slots[cap.identity.slot]
            .entry
            .as_mut()
            .ok_or_else(custody_error)?
            .access = Access::Frozen;
        cap.access = Access::Frozen;
        Ok(cap)
    }

    pub(crate) fn allocate<T>(
        &self,
        cap: &Capability<Domain>,
        domain: Domain,
        layout: RegionLayout<T>,
        bytes: &[u8],
        control: &ExecutionControl,
    ) -> Result<Pointer, ExecutionError> {
        let mut state = self.write_state();
        self.entry(&state, cap, domain, true)?;
        if cap.access != Access::Unique {
            return Err(custody_error());
        }
        let offset = state.slots[cap.identity.slot]
            .entry
            .as_mut()
            .ok_or_else(custody_error)?
            .slab
            .allocate(layout, 1, bytes, control)?;
        Ok(Pointer {
            identity: cap.identity,
            offset,
        })
    }

    pub(crate) fn with_read<Output>(
        &self,
        cap: &Capability<Domain>,
        domain: Domain,
        control: &ExecutionControl,
        body: impl for<'image, 'region> FnOnce(
            RegionRead<'image, 'region>,
        ) -> Result<Output, ExecutionError>,
    ) -> Result<Output, ExecutionError> {
        control.check()?;
        let state = self.read_state();
        let entry = self.entry(&state, cap, domain, false)?;
        body(entry.slab.read())
    }

    pub(crate) fn resolve<T>(
        &self,
        cap: &Capability<Domain>,
        domain: Domain,
        layout: RegionLayout<T>,
        offset: u64,
        control: &ExecutionControl,
    ) -> Result<Pointer, ExecutionError> {
        self.with_read(cap, domain, control, |read| {
            read.resolve(layout, offset, control)?;
            Ok(Pointer {
                identity: cap.identity,
                offset,
            })
        })
    }

    pub(crate) fn read<T, Output>(
        &self,
        cap: &Capability<Domain>,
        domain: Domain,
        layout: RegionLayout<T>,
        pointer: Pointer,
        control: &ExecutionControl,
        body: impl FnOnce(&[u8]) -> Result<Output, ExecutionError>,
    ) -> Result<Output, ExecutionError> {
        if pointer.identity != cap.identity {
            return Err(custody_error());
        }
        self.with_read(cap, domain, control, |read| {
            let pointer = read.resolve(layout, pointer.offset, control)?;
            body(read.read(layout, pointer, control)?)
        })
    }

    pub(crate) fn write<T>(
        &self,
        cap: &Capability<Domain>,
        domain: Domain,
        layout: RegionLayout<T>,
        pointer: Pointer,
        bytes: &[u8],
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        let mut state = self.write_state();
        let entry = self.entry(&state, cap, domain, true)?;
        if cap.access != Access::Unique || pointer.identity != cap.identity {
            return Err(custody_error());
        }
        let range = entry.slab.read().range(layout, pointer.offset, 1)?;
        if range.len() != bytes.len() {
            return Err(image_error());
        }
        let slab = &mut state.slots[cap.identity.slot]
            .entry
            .as_mut()
            .ok_or_else(custody_error)?
            .slab;
        slab.allocation[slab.start + range.start..slab.start + range.end].copy_from_slice(bytes);
        Ok(())
    }

    /// Fresh lexical custody becomes immutable image custody only after digest
    /// and complete layout admission. The caller owns DataStore authority.
    #[cfg(test)]
    #[allow(
        clippy::too_many_arguments,
        reason = "test helper keeps custody, digest and bounded image admission explicit"
    )]
    pub(crate) fn load(
        &self,
        cap: Capability<Domain>,
        domain: Domain,
        image: &[u8],
        expected: persistence::RegionImageDigest,
        maximum_bytes: usize,
        control: &ExecutionControl,
        reserve: impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Capability<Domain>, ExecutionError> {
        {
            let state = self.read_state();
            let entry = self.entry(&state, &cap, domain, true)?;
            if cap.access != Access::Unique || entry.slab.header.cells != 0 {
                return Err(custody_error());
            }
        }
        let image =
            persistence::RegionImage::from_bytes(image, expected, maximum_bytes, control, reserve)?;
        self.activate_image(cap, domain, image, control)
    }

    pub(crate) fn activate_image(
        &self,
        mut cap: Capability<Domain>,
        domain: Domain,
        image: persistence::RegionImage,
        control: &ExecutionControl,
    ) -> Result<Capability<Domain>, ExecutionError> {
        control.check()?;
        let mut state = self.write_state();
        let entry = self.entry(&state, &cap, domain, true)?;
        if cap.access != Access::Unique || entry.slab.header.cells != 0 {
            return Err(custody_error());
        }
        let entry = state.slots[cap.identity.slot]
            .entry
            .as_mut()
            .ok_or_else(custody_error)?;
        entry.slab = image.into_slab();
        entry.access = Access::Frozen;
        cap.access = Access::Frozen;
        Ok(cap)
    }

    /// Lexical cleanup does not depend on where the unique/frozen token moved.
    /// Only evaluator-created scope records can retain this private identity.
    pub(crate) fn close(&self, identity: Identity) -> Result<(), ExecutionError> {
        let mut state = self.write_state();
        if identity.arena != self.identity {
            return Err(custody_error());
        }
        let free = state.free;
        let slot = state
            .slots
            .get_mut(identity.slot)
            .filter(|slot| slot.generation == identity.generation && slot.entry.is_some())
            .ok_or_else(custody_error)?;
        drop(slot.entry.take());
        slot.next_free = free;
        state.free = Some(identity.slot);
        state.live -= 1;
        Ok(())
    }
}

fn slot_capacity<Domain>(length: usize) -> Result<usize, ExecutionError> {
    let required = length.checked_add(1).ok_or_else(capacity_error)?;
    std::alloc::Layout::array::<Slot<Domain>>(required).map_err(|_| capacity_error())?;
    Ok(required
        .checked_next_power_of_two()
        .filter(|capacity| std::alloc::Layout::array::<Slot<Domain>>(*capacity).is_ok())
        .unwrap_or(required))
}

fn custody_error() -> ExecutionError {
    invalid(
        "region_custody",
        "region capability is foreign, closed, copied or in the wrong mode",
    )
}

#[cfg(test)]
mod tests;
