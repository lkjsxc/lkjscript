mod access;
mod accounting;
mod borrow;
mod cloning;
mod destination;
mod destination_finish;
mod destination_storage;
mod error;
mod events;
mod lifecycle;
mod model;
mod object_slab;
mod release;
mod static_lifecycle;
mod validation;

#[path = "../sealed/value_runtime/destination.rs"]
mod sealed_destination;
#[path = "../sealed/value_runtime/lifecycle.rs"]
mod sealed_lifecycle;
#[path = "../sealed/value_runtime/model.rs"]
mod sealed_model;
#[path = "../sealed/value_runtime/object_slab.rs"]
mod sealed_object_slab;
#[path = "../sealed/value_runtime/release.rs"]
mod sealed_release;

use std::collections::VecDeque;

pub use accounting::{StructuralExportAccounting, StructuralRuntimeAccounting};
pub use error::{StructuralInitializationFailure, StructuralPublishFailure, StructuralValueError};
pub use events::{
    StructuralEvent, StructuralEventKind, StructuralEventLog, StructuralValueRuntimeMetrics,
};
pub use model::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StaticArtifactPayload,
    StaticStructuralArtifact, StaticStructuralLeaf, StructuralDestinationKey, StructuralFieldPath,
    StructuralKind, StructuralProjection, StructuralType, StructuralViewKey,
};
pub use release::DestinationCleanupReport;
pub use sealed_model::{StructuralDisposeReport, StructuralOwnerKind, StructuralSealResult};

use super::image::{
    LocalNodeId, StructuralImage, StructuralNode, StructuralNodePayload, TreeFacts,
};
use super::{
    RootKey, StructuralRootTable, StructuralRootTableError, StructuralRuntime, StructuralValueKey,
};
use borrow::ViewSlot;
use destination::DestinationSlot;
use object_slab::{ObjectSlab, StructuralObject};

#[derive(Debug)]
pub struct StructuralValueRuntime {
    runtime: StructuralRuntime,
    roots: StructuralRootTable,
    objects: ObjectSlab,
    destinations: Vec<DestinationSlot>,
    free_destinations: Vec<u32>,
    views: Vec<ViewSlot>,
    free_views: Vec<u32>,
    metrics: StructuralValueRuntimeMetrics,
    events: StructuralEventLog,
    cleanup_reports: VecDeque<DestinationCleanupReport>,
    cleanup_sequence: u64,
    allocation_events: u64,
}

impl StructuralValueRuntime {
    pub const fn identity(&self) -> super::StructuralRuntimeId {
        self.runtime.identity()
    }

    pub const fn metrics(&self) -> StructuralValueRuntimeMetrics {
        self.metrics
    }

    pub const fn domain_metrics(&self) -> super::StructuralRuntimeMetrics {
        self.runtime.metrics()
    }

    pub const fn events(&self) -> &StructuralEventLog {
        &self.events
    }

    pub(super) fn record(&mut self, kind: StructuralEventKind, subject: u32, amount: u64) {
        if self.events.record(kind, subject, amount) {
            self.metrics.events_overwritten = self.metrics.events_overwritten.saturating_add(1);
        }
    }

    pub(super) fn next_allocation_event(&self) -> Result<u64, StructuralValueError> {
        self.allocation_events
            .checked_add(1)
            .ok_or(StructuralValueError::ArithmeticOverflow)
    }

    pub(super) fn resolve_root(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<RootKey, StructuralValueError> {
        match self
            .roots
            .root(key, expected.layout, expected.semantic_type)
        {
            Ok(root) => Ok(root),
            Err(error) => {
                if matches!(
                    error,
                    StructuralRootTableError::StaleRoot
                        | StructuralRootTableError::MovedRoot
                        | StructuralRootTableError::DroppedRoot
                        | StructuralRootTableError::RetiredRoot
                ) {
                    self.metrics.stale_rejections = self.metrics.stale_rejections.saturating_add(1);
                    self.record(StructuralEventKind::Stale, key.slot(), 0);
                }
                Err(error.into())
            }
        }
    }
}
