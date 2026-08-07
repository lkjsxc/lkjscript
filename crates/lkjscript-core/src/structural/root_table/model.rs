use std::collections::{HashMap, HashSet};
use std::num::NonZeroU64;

use super::super::{RootKey, StructuralRuntimeId};
use super::{StructuralRootTableError, StructuralRootTableStats};

macro_rules! opaque_key {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(NonZeroU64);

        impl $name {
            pub const fn from_word(word: u64) -> Option<Self> {
                match NonZeroU64::new(word) {
                    Some(word) => Some(Self(word)),
                    None => None,
                }
            }

            pub const fn get(self) -> u64 {
                self.0.get()
            }

            pub(super) const fn from_token(token: NonZeroU64) -> Self {
                Self(token)
            }
        }
    };
}

opaque_key!(StructuralValueKey);
opaque_key!(StructuralBorrowKey);

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
    pub shared_loans: u64,
    pub exclusive_loan: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum RootSlot {
    Vacant {
        generation: NonZeroU64,
        previous: Option<(StructuralValueKey, NonZeroU64, TerminalState)>,
    },
    Live {
        generation: NonZeroU64,
        key: StructuralValueKey,
        value: LiveRoot,
    },
    Retired {
        generation: NonZeroU64,
        key: StructuralValueKey,
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
        generation: NonZeroU64,
    },
    Live {
        generation: NonZeroU64,
        key: StructuralBorrowKey,
        value: LiveLoan,
    },
    Retired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StructuralTokenKind {
    Value,
    Borrow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct StructuralTokenRecord {
    pub kind: StructuralTokenKind,
    pub slot: u64,
    pub generation: NonZeroU64,
}

#[derive(Debug)]
pub struct StructuralRootTable {
    pub(super) runtime: StructuralRuntimeId,
    pub(super) roots: Vec<RootSlot>,
    pub(super) free_roots: Vec<u64>,
    pub(super) exclusive_roots: HashSet<RootKey>,
    pub(super) loans: Vec<LoanSlot>,
    pub(super) free_loans: Vec<u64>,
    pub(super) tokens: HashMap<u64, StructuralTokenRecord>,
    pub(super) next_token: Option<NonZeroU64>,
    pub(super) stats: StructuralRootTableStats,
}

impl StructuralRootTable {
    pub(super) fn allocate_value_token(
        &mut self,
        slot: u64,
        generation: NonZeroU64,
    ) -> Result<StructuralValueKey, StructuralRootTableError> {
        let token = self.allocate_token(StructuralTokenKind::Value, slot, generation)?;
        Ok(StructuralValueKey::from_token(token))
    }

    pub(super) fn allocate_borrow_token(
        &mut self,
        slot: u64,
        generation: NonZeroU64,
    ) -> Result<StructuralBorrowKey, StructuralRootTableError> {
        let token = self.allocate_token(StructuralTokenKind::Borrow, slot, generation)?;
        Ok(StructuralBorrowKey::from_token(token))
    }

    pub(super) fn value_token(
        &self,
        key: StructuralValueKey,
    ) -> Result<StructuralTokenRecord, StructuralRootTableError> {
        self.token(key.get(), StructuralTokenKind::Value)
            .ok_or(StructuralRootTableError::StaleRoot)
    }

    pub(super) fn borrow_token(
        &self,
        key: StructuralBorrowKey,
    ) -> Result<StructuralTokenRecord, StructuralRootTableError> {
        self.token(key.get(), StructuralTokenKind::Borrow)
            .ok_or(StructuralRootTableError::StaleLoan)
    }

    fn allocate_token(
        &mut self,
        kind: StructuralTokenKind,
        slot: u64,
        generation: NonZeroU64,
    ) -> Result<NonZeroU64, StructuralRootTableError> {
        self.tokens
            .try_reserve(1)
            .map_err(|_| StructuralRootTableError::AllocationFailed)?;
        let token = self
            .next_token
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        self.next_token = token.get().checked_add(1).and_then(NonZeroU64::new);
        if self.tokens.contains_key(&token.get()) {
            return Err(StructuralRootTableError::InvariantViolation);
        }
        self.tokens.insert(
            token.get(),
            StructuralTokenRecord {
                kind,
                slot,
                generation,
            },
        );
        Ok(token)
    }

    fn token(&self, token: u64, kind: StructuralTokenKind) -> Option<StructuralTokenRecord> {
        self.tokens
            .get(&token)
            .copied()
            .filter(|record| record.kind == kind)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn opaque_tokens_map_high_wide_slot_and_generation_without_allocation_geometry() {
        let runtime = StructuralRuntimeId::new(NonZeroU64::MIN);
        let mut table = StructuralRootTable::new(runtime).expect("root table");
        let high = u64::from(u32::MAX) + 41;
        table.next_token = NonZeroU64::new(high);
        let generation = NonZeroU64::new(high + 1).expect("generation");
        let key = table
            .allocate_value_token(high + 2, generation)
            .expect("high opaque token");
        let record = table.value_token(key).expect("mapped token");
        assert_eq!(key.get(), high);
        assert_eq!(record.slot, high + 2);
        assert_eq!(record.generation, generation);
    }
}
