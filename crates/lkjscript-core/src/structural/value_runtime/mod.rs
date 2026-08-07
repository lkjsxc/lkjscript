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

use std::collections::{HashMap, VecDeque};
use std::num::NonZeroU64;

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
    free_destinations: Vec<u64>,
    views: Vec<ViewSlot>,
    free_views: Vec<u64>,
    private_tokens: HashMap<u64, PrivateTokenRecord>,
    next_private_token: Option<NonZeroU64>,
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

    pub(super) fn record(&mut self, kind: StructuralEventKind, subject: u64, amount: u64) {
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
                    self.record(StructuralEventKind::Stale, key.get(), 0);
                }
                Err(error.into())
            }
        }
    }

    fn allocate_private_token(
        &mut self,
        kind: PrivateTokenKind,
        slot: u64,
        generation: NonZeroU64,
    ) -> Result<NonZeroU64, StructuralValueError> {
        self.private_tokens.try_reserve(1)?;
        let token = self
            .next_private_token
            .ok_or(StructuralValueError::ArithmeticOverflow)?;
        self.next_private_token = token.get().checked_add(1).and_then(NonZeroU64::new);
        if self.private_tokens.contains_key(&token.get()) {
            return Err(StructuralValueError::InvariantViolation);
        }
        self.private_tokens.insert(
            token.get(),
            PrivateTokenRecord {
                kind,
                slot,
                generation,
            },
        );
        Ok(token)
    }

    fn destination_token(
        &self,
        key: StructuralDestinationKey,
    ) -> Result<PrivateTokenRecord, StructuralValueError> {
        self.private_token(key.get(), PrivateTokenKind::Destination)
            .ok_or(StructuralValueError::StaleDestination)
    }

    fn view_token(
        &self,
        key: StructuralViewKey,
    ) -> Result<PrivateTokenRecord, StructuralValueError> {
        self.private_token(key.get(), PrivateTokenKind::View)
            .ok_or(StructuralValueError::StaleView)
    }

    fn private_token(&self, token: u64, kind: PrivateTokenKind) -> Option<PrivateTokenRecord> {
        self.private_tokens
            .get(&token)
            .copied()
            .filter(|record| record.kind == kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PrivateTokenKind {
    Destination,
    View,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PrivateTokenRecord {
    kind: PrivateTokenKind,
    slot: u64,
    generation: NonZeroU64,
}
