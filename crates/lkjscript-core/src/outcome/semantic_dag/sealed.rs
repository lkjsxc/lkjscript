mod cells;
mod export;
mod import;
mod model;
mod ownership;
mod planning;
mod runtime;
#[cfg(test)]
mod tests;
mod validated;
mod validated_types;

pub use model::{
    SealedSemanticDagBorrow, SealedSemanticDagBorrowFailure, SealedSemanticDagError,
    SealedSemanticDagFailure, SealedSemanticDagMetrics, SealedSemanticDagOwner,
    SealedSemanticDagReleaseFailure, SealedSemanticDagReleaseReport,
};
pub use runtime::SealedSemanticDagRuntime;

use super::SemanticDagType;
use crate::structural::SealedRegionStore;
use cells::SealedDagCell;

const SEALED_DAG_BYTE_CHUNK: usize = 32;

#[derive(Debug)]
struct TypedSealedDagStore {
    value_type: SemanticDagType,
    store: SealedRegionStore<SealedDagCell, ()>,
}
