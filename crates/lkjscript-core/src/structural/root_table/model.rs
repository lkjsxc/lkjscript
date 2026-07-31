use std::num::NonZeroU32;

use super::super::{RootKey, StructuralRuntimeId};
use super::{StructuralRootTableLimits, StructuralRootTableStats};

macro_rules! packed_key {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u64);

        impl $name {
            pub const fn from_word(word: u64) -> Option<Self> {
                if word >> 32 == 0 {
                    None
                } else {
                    Some(Self(word))
                }
            }

            pub const fn get(self) -> u64 {
                self.0
            }

            pub const fn slot(self) -> u32 {
                self.get() as u32
            }

            pub const fn generation(self) -> u32 {
                (self.get() >> 32) as u32
            }

            pub(super) const fn from_parts(slot: u32, generation: NonZeroU32) -> Self {
                Self(((generation.get() as u64) << 32) | slot as u64)
            }
        }
    };
}

packed_key!(StructuralValueKey);
packed_key!(StructuralBorrowKey);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralRootOwnership {
    Owned,
    Static,
    SealedShared,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralRootState {
    Owned,
    BorrowedShared,
    BorrowedExclusive,
    Static,
    SealedShared,
    Moved,
    Dropped,
    Retired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralBorrow {
    key: StructuralBorrowKey,
    root: StructuralValueKey,
    exclusive: bool,
}

impl StructuralBorrow {
    pub const fn key(self) -> StructuralBorrowKey {
        self.key
    }

    pub const fn root(self) -> StructuralValueKey {
        self.root
    }

    pub const fn is_exclusive(self) -> bool {
        self.exclusive
    }

    pub(super) const fn new(
        key: StructuralBorrowKey,
        root: StructuralValueKey,
        exclusive: bool,
    ) -> Self {
        Self {
            key,
            root,
            exclusive,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminalState {
    Moved,
    Dropped,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LiveRoot {
    pub root: RootKey,
    pub ownership: StructuralRootOwnership,
    pub shared_loans: u32,
    pub exclusive_loan: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum RootSlot {
    Vacant {
        generation: NonZeroU32,
        previous: Option<(NonZeroU32, TerminalState)>,
    },
    Live {
        generation: NonZeroU32,
        value: LiveRoot,
    },
    Retired {
        generation: NonZeroU32,
    },
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LiveLoan {
    pub root: StructuralValueKey,
    pub exclusive: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum LoanSlot {
    Vacant {
        generation: NonZeroU32,
    },
    Live {
        generation: NonZeroU32,
        value: LiveLoan,
    },
    Retired,
}

#[derive(Debug)]
pub struct StructuralRootTable {
    pub(super) runtime: StructuralRuntimeId,
    pub(super) limits: StructuralRootTableLimits,
    pub(super) roots: Vec<RootSlot>,
    pub(super) free_roots: Vec<u32>,
    pub(super) loans: Vec<LoanSlot>,
    pub(super) free_loans: Vec<u32>,
    pub(super) stats: StructuralRootTableStats,
}
