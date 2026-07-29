mod error;
mod identity;
mod ledger;
mod limits;
mod metrics;
mod pool;
mod region;
mod root_table;
mod runtime;
mod sealed;
mod unique;

pub use error::{StructuralError, StructuralLimit};
pub use identity::{
    DomainClass, DomainKey, LayoutIdentity, RootClass, RootKey, SemanticTypeIdentity,
    StructuralRuntimeId,
};
pub use limits::StructuralLimits;
pub use metrics::{PoolMetrics, RegionMetrics, SealedRegionMetrics, StructuralRuntimeMetrics};
pub use pool::{PoolId, PoolPartition, TypedPool};
pub use region::{RegionOwner, RegionRef, RegionReleaseReport, RegionStore};
pub use root_table::{
    StructuralBorrow, StructuralBorrowKey, StructuralRootOwnership, StructuralRootState,
    StructuralRootTable, StructuralRootTableError, StructuralRootTableLimit,
    StructuralRootTableLimits, StructuralRootTableStats, StructuralValueKey,
};
pub use runtime::StructuralRuntime;
pub use sealed::{
    SealFailure, SealedBorrow, SealedBuilder, SealedOwner, SealedRef, SealedRegionStore,
    SealedReleaseReport, SealedUpgrade, WeakSealedRef,
};
pub use unique::{
    ByteVectorKey, BytesKey, InvalidUniqueKeyWord, InvalidUniqueStoreLimits, PathKey, StaticBytes,
    UniqueKeyWord, UniqueLayout, UniqueStore, UniqueStoreError, UniqueStoreId, UniqueStoreLeak,
    UniqueStoreLimits, UniqueStoreStats,
};
