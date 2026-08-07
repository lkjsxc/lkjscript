use std::num::NonZeroU64;

use crate::Value;

use super::super::StructuralValueKey;
use super::destination::{DestinationRecord, DestinationShape, DestinationSlot};
use super::{
    DestinationCleanupReport, PrivateTokenKind, StructuralDestinationKey, StructuralEventKind,
    StructuralImage, StructuralObject, StructuralValueError, StructuralValueRuntime, TreeFacts,
};

impl StructuralValueRuntime {
    pub fn finish_destination(
        &mut self,
        key: StructuralDestinationKey,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        let (image, facts) = self.complete_destination_image(key)?;
        match self.publish_image(image, facts) {
            Ok(root) => {
                self.retire_destination(key)?;
                self.metrics.destinations_completed =
                    self.metrics.destinations_completed.saturating_add(1);
                self.metrics.live_destinations = self.metrics.live_destinations.saturating_sub(1);
                self.record(StructuralEventKind::DestinationComplete, key.get(), 0);
                Ok(root)
            }
            Err(failure) => Err(failure.0),
        }
    }

    pub fn finish_destination_value(
        &mut self,
        key: StructuralDestinationKey,
    ) -> Result<Value, StructuralValueError> {
        self.finish_destination(key)
            .map(Value::from_structural_root)
    }

    pub(super) fn complete_destination_image(
        &self,
        key: StructuralDestinationKey,
    ) -> Result<(StructuralImage, TreeFacts), StructuralValueError> {
        let record = self.destination(key)?;
        if record.values.iter().any(Option::is_none) {
            return Err(StructuralValueError::IncompleteDestination);
        }
        let facts = TreeFacts {
            nodes: 1,
            ..TreeFacts::default()
        }
        .checked_add(record.total)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
        let mut children = Vec::new();
        children.try_reserve_exact(record.values.len())?;
        for value in &record.values {
            children.push(
                value
                    .as_ref()
                    .ok_or(StructuralValueError::InvariantViolation)?,
            );
        }
        let image = StructuralImage::merge(
            record.value_type,
            match record.shape {
                DestinationShape::Product => None,
                DestinationShape::Enum(tag) => Some(tag),
            },
            &children,
            facts,
        )?;
        Ok((image, facts))
    }

    pub fn abort_destination(
        &mut self,
        key: StructuralDestinationKey,
    ) -> Result<DestinationCleanupReport, StructuralValueError> {
        let mut record = self.retire_destination(key)?;
        let initialized_fields = record.order.len();
        record.order.reverse();
        let mut report = DestinationCleanupReport {
            sequence: self.cleanup_sequence,
            initialized_fields,
            cleanup_order: record.order,
            nodes_released: 0,
            bytes_released: 0,
        };
        self.cleanup_sequence = self.cleanup_sequence.saturating_add(1);
        for &field in &report.cleanup_order {
            let index = field;
            let image = record.values[index]
                .take()
                .ok_or(StructuralValueError::InvariantViolation)?;
            let facts = record.facts[index]
                .take()
                .ok_or(StructuralValueError::InvariantViolation)?;
            report.nodes_released = report.nodes_released.saturating_add(facts.nodes);
            report.bytes_released = report.bytes_released.saturating_add(facts.bytes);
            self.release_image(image, facts);
        }
        self.metrics.destinations_aborted = self.metrics.destinations_aborted.saturating_add(1);
        self.metrics.live_destinations = self.metrics.live_destinations.saturating_sub(1);
        self.metrics.destination_cleanup_work = self
            .metrics
            .destination_cleanup_work
            .saturating_add(report.nodes_released);
        let initialized_fields = u64::try_from(report.initialized_fields)
            .map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        self.record(
            StructuralEventKind::DestinationAbort,
            key.get(),
            initialized_fields,
        );
        self.record(
            StructuralEventKind::DestinationCleanup,
            key.get(),
            report.nodes_released,
        );
        // Diagnostic retention must never prevent cleanup. If retaining this
        // report cannot grow, cleanup has still completed and the report is omitted.
        if self.cleanup_reports.try_reserve(1).is_ok() {
            self.cleanup_reports.push_back(report.clone());
        }
        Ok(report)
    }

    pub(super) fn allocate_destination(
        &mut self,
        record: DestinationRecord,
    ) -> Result<StructuralDestinationKey, StructuralValueError> {
        let (slot, generation, reused) = if let Some(&slot) = self.free_destinations.last() {
            let index =
                usize::try_from(slot).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
            let DestinationSlot::Vacant(generation) = self
                .destinations
                .get(index)
                .ok_or(StructuralValueError::InvariantViolation)?
            else {
                return Err(StructuralValueError::InvariantViolation);
            };
            (slot, *generation, true)
        } else {
            self.destinations.try_reserve(1)?;
            let slot = u64::try_from(self.destinations.len())
                .map_err(|_| StructuralValueError::ArithmeticOverflow)?;
            (slot, NonZeroU64::MIN, false)
        };
        let token = self.allocate_private_token(PrivateTokenKind::Destination, slot, generation)?;
        let key = StructuralDestinationKey::from_token(token);
        let index = usize::try_from(slot).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        if reused {
            if self.free_destinations.pop() != Some(slot) {
                return Err(StructuralValueError::InvariantViolation);
            }
            self.destinations[index] = DestinationSlot::Live {
                generation,
                key,
                record,
            };
        } else {
            self.destinations.push(DestinationSlot::Live {
                generation,
                key,
                record,
            });
        }
        Ok(key)
    }

    pub(super) fn preflight_root_field(
        &mut self,
        key: StructuralDestinationKey,
        field: usize,
        root_key: StructuralValueKey,
        expected: super::StructuralType,
    ) -> Result<(), StructuralValueError> {
        let root = self.resolve_root(root_key, expected)?;
        let StructuralObject::Owned { image, facts } = self.objects.get(root)? else {
            return Err(StructuralValueError::WrongOwnership);
        };
        self.preflight_field(key, field, image.root().value_type(), *facts)
    }

    pub(super) fn retire_destination(
        &mut self,
        key: StructuralDestinationKey,
    ) -> Result<DestinationRecord, StructuralValueError> {
        self.destination(key)?;
        let token = self.destination_token(key)?;
        let index =
            usize::try_from(token.slot).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let next = if token.generation.get() == u64::MAX {
            DestinationSlot::Retired
        } else {
            self.free_destinations.try_reserve(1)?;
            self.free_destinations.push(token.slot);
            DestinationSlot::Vacant(
                token
                    .generation
                    .get()
                    .checked_add(1)
                    .and_then(NonZeroU64::new)
                    .ok_or(StructuralValueError::ArithmeticOverflow)?,
            )
        };
        let DestinationSlot::Live { record, .. } =
            std::mem::replace(&mut self.destinations[index], next)
        else {
            return Err(StructuralValueError::InvariantViolation);
        };
        self.private_tokens.remove(&key.get());
        Ok(record)
    }
}
