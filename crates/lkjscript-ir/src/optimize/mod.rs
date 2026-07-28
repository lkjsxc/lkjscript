mod api;
mod budget;
mod checker;
mod discovery;
mod equality;
mod equality_enums;
mod equality_numeric;
mod equality_ownership;
mod model;
mod passes;
mod reconstruct;
mod shape;

pub use api::{optimize, verify_optimization};
pub(crate) use budget::*;
pub(crate) use checker::*;
pub(crate) use discovery::*;
pub(crate) use equality::*;
pub use model::{
    OptimizationCertificate, OptimizationCertificateRecord, OptimizationEditKind,
    OptimizationError, OptimizationFailureCode, OptimizationLimits, OptimizationStats,
    VerifiedOptimizedProgram,
};
pub use passes::{
    canonical_block_order, constant_fold_and_propagate, copy_propagate, direct_call_resolution,
    effect_aware_dce, empty_block_forwarding, normalize_baseline, simplify_branches,
    unreachable_blocks,
};
pub(crate) use reconstruct::*;
pub(crate) use shape::*;
