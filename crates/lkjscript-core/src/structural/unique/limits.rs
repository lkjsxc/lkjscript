use std::num::NonZeroU32;

use super::InvalidUniqueStoreLimits;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UniqueStoreLimits {
    max_objects: u32,
    max_bytes: u64,
    max_slots: u32,
    max_allocations: u64,
    max_generation: NonZeroU32,
}

impl UniqueStoreLimits {
    pub const fn new(
        max_objects: u32,
        max_bytes: u64,
        max_slots: u32,
        max_allocations: u64,
        max_generation: u32,
    ) -> Result<Self, InvalidUniqueStoreLimits> {
        if max_objects > max_slots {
            return Err(InvalidUniqueStoreLimits::ObjectsExceedSlots);
        }
        let Some(max_generation) = NonZeroU32::new(max_generation) else {
            return Err(InvalidUniqueStoreLimits::ZeroGeneration);
        };
        Ok(Self {
            max_objects,
            max_bytes,
            max_slots,
            max_allocations,
            max_generation,
        })
    }

    pub const fn max_objects(self) -> u32 {
        self.max_objects
    }

    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }

    pub const fn max_slots(self) -> u32 {
        self.max_slots
    }

    pub const fn max_allocations(self) -> u64 {
        self.max_allocations
    }

    pub const fn max_generation(self) -> NonZeroU32 {
        self.max_generation
    }
}
