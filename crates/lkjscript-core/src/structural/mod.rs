mod bookkeeping;
mod error;
mod identity;
mod image;
mod metrics;
mod pool;
mod region;
mod region_product;
mod root_table;
mod runtime;
mod sealed;
mod segmented_list;
mod unique;
mod value_runtime;

pub use error::StructuralError;
pub use identity::{
    product_layout_identity, product_semantic_identity, DomainClass, DomainKey, LayoutIdentity,
    RootClass, RootKey, SemanticTypeIdentity, StructuralRuntimeId,
};
pub use image::{
    CheckedU64Range, LocalNodeId, SemanticChildren, StructuralImage,
    StructuralImageConversionFailure, StructuralNode, StructuralNodePayload, StructuralNodeRecord,
    StructuralNodeView,
};
pub use metrics::{PoolMetrics, RegionMetrics, SealedRegionMetrics, StructuralRuntimeMetrics};
pub use pool::{PoolId, PoolPartition, TypedPool};
pub use region::{RegionOwner, RegionRef, RegionReleaseReport, RegionStore};
pub use region_product::{
    RegionProductArena, RegionProductArenaId, RegionProductError, RegionProductKey,
    RegionProductMetrics,
};
pub use root_table::{
    StructuralBorrow, StructuralBorrowKey, StructuralRootOwnership, StructuralRootState,
    StructuralRootTable, StructuralRootTableError, StructuralRootTableStats, StructuralValueKey,
};
pub use runtime::StructuralRuntime;
pub use sealed::{
    SealFailure, SealedBorrow, SealedBuilder, SealedOwner, SealedRef, SealedRegionStore,
    SealedReleaseReport, SealedUpgrade, WeakSealedRef,
};
pub use segmented_list::{
    SegmentedListArena, SegmentedListArenaId, SegmentedListError, SegmentedListKey,
    SegmentedListLimit, SegmentedListMetrics,
};
pub use unique::{
    ByteVectorKey, BytesKey, InvalidUniqueKeyWord, PathKey, StaticBytes, UniqueKeyWord,
    UniqueLayout, UniqueStore, UniqueStoreError, UniqueStoreId, UniqueStoreLeak, UniqueStoreStats,
};
pub use value_runtime::{
    DestinationCleanupReport, InlineStructuralValue, SemanticPayload, SemanticValue,
    StaticArtifactPayload, StaticStructuralArtifact, StaticStructuralLeaf,
    StructuralDestinationKey, StructuralDisposeReport, StructuralEvent, StructuralEventKind,
    StructuralEventLog, StructuralExportAccounting, StructuralFieldPath,
    StructuralInitializationFailure, StructuralKind, StructuralOwnerKind, StructuralProjection,
    StructuralPublishFailure, StructuralRuntimeAccounting, StructuralSealResult, StructuralType,
    StructuralValueError, StructuralValueRuntime, StructuralValueRuntimeMetrics, StructuralViewKey,
};
