use std::error::Error;
use std::fmt;
use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceTableConfigError {
    ZeroSlots,
    ReservationsExceedSlots,
    OwnedExceedSlots,
    BorrowedExceedSlots,
    ChildrenExceedSlots,
}

impl fmt::Display for ResourceTableConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroSlots => "resource table requires at least one slot",
            Self::ReservationsExceedSlots => "reservation limit exceeds slot limit",
            Self::OwnedExceedSlots => "owned resource limit exceeds slot limit",
            Self::BorrowedExceedSlots => "borrowed resource limit exceeds slot limit",
            Self::ChildrenExceedSlots => "child limit exceeds slot limit",
        };
        formatter.write_str(message)
    }
}

impl Error for ResourceTableConfigError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceTableLimits {
    max_slots: usize,
    max_reserved: usize,
    max_owned: usize,
    max_borrowed: usize,
    max_children_per_parent: usize,
    max_generation: NonZeroU64,
}

impl ResourceTableLimits {
    pub const fn new(
        max_slots: usize,
        max_reserved: usize,
        max_owned: usize,
        max_borrowed: usize,
        max_children_per_parent: usize,
        max_generation: NonZeroU64,
    ) -> Result<Self, ResourceTableConfigError> {
        if max_slots == 0 {
            return Err(ResourceTableConfigError::ZeroSlots);
        }
        if max_reserved > max_slots {
            return Err(ResourceTableConfigError::ReservationsExceedSlots);
        }
        if max_owned > max_slots {
            return Err(ResourceTableConfigError::OwnedExceedSlots);
        }
        if max_borrowed > max_slots {
            return Err(ResourceTableConfigError::BorrowedExceedSlots);
        }
        if max_children_per_parent > max_slots {
            return Err(ResourceTableConfigError::ChildrenExceedSlots);
        }
        Ok(Self {
            max_slots,
            max_reserved,
            max_owned,
            max_borrowed,
            max_children_per_parent,
            max_generation,
        })
    }

    pub const fn max_slots(self) -> usize {
        self.max_slots
    }

    pub const fn max_reserved(self) -> usize {
        self.max_reserved
    }

    pub const fn max_owned(self) -> usize {
        self.max_owned
    }

    pub const fn max_borrowed(self) -> usize {
        self.max_borrowed
    }

    pub const fn max_children_per_parent(self) -> usize {
        self.max_children_per_parent
    }

    pub const fn max_generation(self) -> NonZeroU64 {
        self.max_generation
    }
}
