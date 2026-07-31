use std::collections::VecDeque;

use crate::Value;

use super::super::{
    DomainClass, RootClass, StructuralRootOwnership, StructuralRootTable, StructuralRuntime,
    StructuralValueKey,
};
use super::{
    ObjectSlab, SemanticValue, StructuralEventKind, StructuralEventLog, StructuralObject,
    StructuralPublishFailure, StructuralType, StructuralValueError, StructuralValueRuntime,
    StructuralValueRuntimeLimits,
};

impl StructuralValueRuntime {
    pub fn new(limits: StructuralValueRuntimeLimits) -> Result<Self, StructuralValueError> {
        let limits = limits.validate()?;
        let runtime = StructuralRuntime::new(limits.domains)?;
        let roots = StructuralRootTable::new(runtime.identity(), limits.roots)?;
        let mut release_stack = Vec::new();
        release_stack
            .try_reserve_exact(limits.max_tree_nodes as usize)
            .map_err(|_| StructuralValueError::AllocationFailed)?;
        let mut cleanup_reports = VecDeque::new();
        cleanup_reports
            .try_reserve_exact(limits.max_cleanup_reports as usize)
            .map_err(|_| StructuralValueError::AllocationFailed)?;
        Ok(Self {
            runtime,
            roots,
            objects: ObjectSlab::new(limits),
            destinations: Vec::new(),
            free_destinations: Vec::new(),
            views: Vec::new(),
            free_views: Vec::new(),
            metrics: super::StructuralValueRuntimeMetrics::default(),
            events: StructuralEventLog::new(limits.max_events)?,
            cleanup_reports,
            cleanup_sequence: 1,
            release_stack,
            limits,
        })
    }

    pub fn publish_owned(
        &mut self,
        value: SemanticValue,
    ) -> Result<StructuralValueKey, StructuralPublishFailure> {
        let facts = match self.validate_tree(&value) {
            Ok(facts) => facts,
            Err(error) => return Err(StructuralPublishFailure { error, value }),
        };
        let domain = match self.runtime.allocate(DomainClass::Unique) {
            Ok(domain) => domain,
            Err(error) => {
                return Err(StructuralPublishFailure {
                    error: error.into(),
                    value,
                })
            }
        };
        let object = StructuralObject::Owned { value, facts };
        let (root, reused) = match self.objects.insert(domain, RootClass::UniquePublic, object) {
            Ok(root) => root,
            Err(failure) => {
                let (error, object) = *failure;
                self.runtime.rollback_allocation(domain);
                return Err(StructuralPublishFailure {
                    error,
                    value: owned_value(object),
                });
            }
        };
        match self.roots.publish(root, StructuralRootOwnership::Owned) {
            Ok(key) => {
                self.note_publication(facts);
                self.note_slot_reuse(reused);
                self.record(
                    StructuralEventKind::Allocate,
                    root.slot(),
                    u64::from(facts.nodes),
                );
                self.record(StructuralEventKind::Publish, key.slot(), 0);
                Ok(key)
            }
            Err(error) => {
                let object = self.objects.rollback_insert(root, reused);
                self.runtime.rollback_allocation(domain);
                Err(StructuralPublishFailure {
                    error: error.into(),
                    value: owned_value(object),
                })
            }
        }
    }

    pub fn publish_value(
        &mut self,
        value: SemanticValue,
    ) -> Result<Value, StructuralPublishFailure> {
        self.publish_owned(value).map(Value::from_structural_root)
    }

    pub fn move_owned(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_owned_root(root, expected)?;
        self.runtime.preflight_release(&[root.domain()])?;
        let root = self.roots.take_owned(key)?;
        match self.roots.publish(root, StructuralRootOwnership::Owned) {
            Ok(next) => {
                self.metrics.moves = self.metrics.moves.saturating_add(1);
                self.record(
                    StructuralEventKind::Move,
                    key.slot(),
                    u64::from(next.slot()),
                );
                Ok(next)
            }
            Err(error) => {
                let object = self.objects.take(root)?;
                self.runtime.release(root.domain())?;
                let StructuralObject::Owned { value, facts } = object else {
                    return Err(StructuralValueError::InvariantViolation);
                };
                self.note_object_removed(facts);
                self.release_tree(value, facts);
                Err(error.into())
            }
        }
    }

    pub fn drop_owned(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<(), StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_owned_root(root, expected)?;
        self.runtime.preflight_release(&[root.domain()])?;
        let root = self.roots.drop_owned(key)?;
        let StructuralObject::Owned { value, facts } = self.objects.take(root)? else {
            return Err(StructuralValueError::InvariantViolation);
        };
        self.runtime.release(root.domain())?;
        self.note_object_removed(facts);
        self.metrics.drops = self.metrics.drops.saturating_add(1);
        self.record(
            StructuralEventKind::Drop,
            key.slot(),
            u64::from(facts.nodes),
        );
        self.release_tree(value, facts);
        Ok(())
    }

    pub fn export_semantic(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<SemanticValue, StructuralValueError> {
        let value = self.take_owned_value(key, expected)?;
        self.record(StructuralEventKind::Export, key.slot(), 0);
        Ok(value)
    }

    pub(super) fn take_owned_value(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<SemanticValue, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_owned_root(root, expected)?;
        self.runtime.preflight_release(&[root.domain()])?;
        let root = self.roots.take_owned(key)?;
        let StructuralObject::Owned { value, facts } = self.objects.take(root)? else {
            return Err(StructuralValueError::InvariantViolation);
        };
        self.runtime.release(root.domain())?;
        self.note_object_removed(facts);
        self.metrics.moves = self.metrics.moves.saturating_add(1);
        Ok(value)
    }
}

fn owned_value(object: StructuralObject) -> SemanticValue {
    match object {
        StructuralObject::Owned { value, .. } => value,
        StructuralObject::Static(_) => unreachable!("owned publication object"),
    }
}
