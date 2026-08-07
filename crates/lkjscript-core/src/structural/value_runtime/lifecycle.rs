use std::collections::VecDeque;

use crate::Value;

use super::super::image::{discard_semantic, prepare_discard};
use super::super::{
    DomainClass, RootClass, StructuralRootOwnership, StructuralRootTable, StructuralRuntime,
    StructuralValueKey,
};
use super::{
    ObjectSlab, SemanticValue, StructuralEventKind, StructuralEventLog, StructuralImage,
    StructuralObject, StructuralPublishFailure, StructuralType, StructuralValueError,
    StructuralValueRuntime, TreeFacts,
};

impl StructuralValueRuntime {
    pub fn new() -> Result<Self, StructuralValueError> {
        let runtime = StructuralRuntime::new()?;
        let roots = StructuralRootTable::new(runtime.identity())?;
        Ok(Self {
            runtime,
            roots,
            objects: ObjectSlab::new(),
            destinations: Vec::new(),
            free_destinations: Vec::new(),
            views: Vec::new(),
            free_views: Vec::new(),
            metrics: super::StructuralValueRuntimeMetrics::default(),
            events: StructuralEventLog::new(),
            cleanup_reports: VecDeque::new(),
            cleanup_sequence: 1,
            allocation_events: 0,
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
        let mut discard = match prepare_discard(facts) {
            Ok(stack) => stack,
            Err(error) => return Err(StructuralPublishFailure { error, value }),
        };
        let image = match StructuralImage::build(&value, facts) {
            Ok(image) => image,
            Err(error) => return Err(StructuralPublishFailure { error, value }),
        };
        match self.publish_image(image, facts) {
            Ok(key) => {
                discard_semantic(value, &mut discard);
                Ok(key)
            }
            Err(failure) => Err(StructuralPublishFailure {
                error: failure.0,
                value,
            }),
        }
    }

    pub fn publish_value(
        &mut self,
        value: SemanticValue,
    ) -> Result<Value, StructuralPublishFailure> {
        self.publish_owned(value).map(Value::from_structural_root)
    }

    pub(super) fn publish_image(
        &mut self,
        image: StructuralImage,
        facts: TreeFacts,
    ) -> Result<StructuralValueKey, Box<(StructuralValueError, StructuralImage)>> {
        if let Err(error) = image.validate(facts) {
            return Err(Box::new((error, image)));
        }
        let next_allocation = match self.next_allocation_event() {
            Ok(next) => next,
            Err(error) => return Err(Box::new((error, image))),
        };
        let domain = match self.runtime.allocate(DomainClass::Unique) {
            Ok(domain) => domain,
            Err(error) => return Err(Box::new((error.into(), image))),
        };
        let object = StructuralObject::Owned { image, facts };
        let (root, reused) = match self.objects.insert(domain, RootClass::UniquePublic, object) {
            Ok(root) => root,
            Err(failure) => {
                let (error, object) = *failure;
                self.runtime.rollback_allocation(domain);
                return Err(Box::new((error, owned_image(object))));
            }
        };
        match self.roots.publish(root, StructuralRootOwnership::Owned) {
            Ok(key) => {
                self.allocation_events = next_allocation;
                self.note_publication(facts);
                self.note_slot_reuse(reused);
                self.record(StructuralEventKind::Allocate, root.slot(), facts.nodes);
                self.record(StructuralEventKind::Publish, key.slot(), 0);
                Ok(key)
            }
            Err(error) => {
                let object = self.objects.rollback_insert(root, reused);
                self.runtime.rollback_allocation(domain);
                Err(Box::new((error.into(), owned_image(object))))
            }
        }
    }

    pub fn move_owned(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_owned_root(root, expected)?;
        self.runtime.preflight_release(&[root.domain()])?;
        self.objects.preflight_take(root)?;
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
                let StructuralObject::Owned { image, facts } = object else {
                    return Err(StructuralValueError::InvariantViolation);
                };
                self.note_object_removed(facts);
                self.release_image(image, facts);
                Err(error.into())
            }
        }
    }

    pub fn drop_owned(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<(), StructuralValueError> {
        let (image, facts) = self.drop_owned_image(key, expected)?;
        self.metrics.drops = self.metrics.drops.saturating_add(1);
        self.record(StructuralEventKind::Drop, key.slot(), facts.nodes);
        self.release_image(image, facts);
        Ok(())
    }

    pub fn export_semantic(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<SemanticValue, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_owned_root(root, expected)?;
        let StructuralObject::Owned { image, .. } = self.objects.get(root)? else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        let value = image.to_semantic()?;
        let (image, _) = self.take_owned_image(key, expected)?;
        drop(image);
        self.record(StructuralEventKind::Export, key.slot(), 0);
        Ok(value)
    }
}

fn owned_image(object: StructuralObject) -> StructuralImage {
    match object {
        StructuralObject::Owned { image, .. } => image,
        StructuralObject::Sealed { .. } | StructuralObject::Static(_) => {
            unreachable!("owned publication object")
        }
    }
}
