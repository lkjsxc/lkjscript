use std::fmt;

use lkjscript_core::{CapabilityKind, ResourceKind};

pub const HIR_MEMORY_PLAN_SCHEMA: &str = "lkjscript.hir-memory-plan";

macro_rules! dense_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            pub(crate) const fn new(raw: u32) -> Self {
                Self(raw)
            }

            pub const fn raw(self) -> u32 {
                self.0
            }

            pub fn index(self) -> Option<usize> {
                usize::try_from(self.0).ok()
            }
        }
    };
}

dense_id!(MemoryFunctionId);
dense_id!(MemoryExpressionId);
dense_id!(MemoryEntryId);
dense_id!(MemoryUseId);
dense_id!(MemoryConstantId);
dense_id!(MemoryCallId);
dense_id!(MemoryObligationId);
dense_id!(MemoryDropGlueId);
dense_id!(MemoryTypeFactId);
dense_id!(MemoryDestinationId);
dense_id!(MemoryBorrowScopeId);
dense_id!(MemoryDropPathId);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MemoryPlanId([u8; 32]);

impl MemoryPlanId {
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        lkjscript_contracts::ContractDigest::from_bytes(self.0).to_hex()
    }
}

impl fmt::Display for MemoryPlanId {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(&self.to_hex())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryMultiplicity {
    Copy,
    ImmutableValue,
    Affine,
    Borrowed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryAliasing {
    Unique,
    BorrowedShared,
    BorrowedExclusive,
    StaticShared,
    LegacyTracedShared,
    External,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryEscape {
    Local,
    Caller,
    Returned,
    Captured,
    Runtime,
    Static,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryDestruction {
    Trivial,
    EndBorrow,
    DropGlue,
    ExternalClose,
    LegacyTraced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryIdentity {
    Value,
    ExternalResource,
    LegacyObject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryPortability {
    Portable,
    WorkerLocal,
    ProcessLocal,
    LinuxHost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryContention {
    None,
    SingleOwner,
    ImmutableShared,
    LegacyShared,
    ProviderSerialized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryAllocationFailure {
    Impossible,
    Trap,
    StructuredOutcome,
    TrapOrOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryMode {
    pub multiplicity: MemoryMultiplicity,
    pub aliasing: MemoryAliasing,
    pub escape: MemoryEscape,
    pub domain: MemoryDomain,
    pub destruction: MemoryDestruction,
    pub identity: MemoryIdentity,
    pub portability: MemoryPortability,
    pub contention: MemoryContention,
    pub allocation_failure: MemoryAllocationFailure,
}

include!("model/types.rs");
include!("model/authority.rs");
include!("model/records.rs");
include!("model/obligations.rs");
