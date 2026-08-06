use std::num::NonZeroU32;

use crate::Value;

use super::super::StructuralValueKey;
use super::destination::{DestinationRecord, DestinationShape, DestinationSlot};
use super::{
    DestinationCleanupReport, StructuralDestinationKey, StructuralEventKind, StructuralImage,
    StructuralObject, StructuralValueError, StructuralValueLimit, StructuralValueRuntime,
    TreeFacts,
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
                self.record(StructuralEventKind::DestinationComplete, key.slot(), 0);
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
            self.limits,
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
            .saturating_add(u64::from(report.nodes_released));
        self.record(
            StructuralEventKind::DestinationAbort,
            key.slot(),
            u64::try_from(report.initialized_fields).unwrap_or(u64::MAX),
        );
        self.record(
            StructuralEventKind::DestinationCleanup,
            key.slot(),
            u64::from(report.nodes_released),
        );
        if self.cleanup_reports.len() == self.limits.max_cleanup_reports as usize {
            self.cleanup_reports.pop_front();
        }
        self.cleanup_reports.push_back(report.clone());
        Ok(report)
    }

    pub(super) fn allocate_destination(
        &mut self,
        record: DestinationRecord,
    ) -> Result<StructuralDestinationKey, StructuralValueError> {
        self.free_destinations.try_reserve(1)?;
        let (slot, generation) = if let Some(slot) = self.free_destinations.pop() {
            let DestinationSlot::Vacant(generation) = self.destinations[slot as usize] else {
                return Err(StructuralValueError::InvariantViolation);
            };
            (slot, generation)
        } else {
            let slot = u32::try_from(self.destinations.len())
                .map_err(|_| StructuralValueError::ArithmeticOverflow)?;
            if slot >= self.limits.max_destinations {
                return Err(StructuralValueError::LimitExceeded(
                    StructuralValueLimit::Destinations,
                ));
            }
            self.destinations.try_reserve(1)?;
            self.destinations
                .push(DestinationSlot::Vacant(NonZeroU32::MIN));
            (slot, NonZeroU32::MIN)
        };
        self.destinations[slot as usize] = DestinationSlot::Live { generation, record };
        Ok(StructuralDestinationKey::new(slot, generation))
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
        let next = if key.generation() >= self.limits.max_generation {
            DestinationSlot::Retired
        } else {
            self.free_destinations.push(key.slot());
            DestinationSlot::Vacant(
                NonZeroU32::new(key.generation() + 1)
                    .ok_or(StructuralValueError::InvariantViolation)?,
            )
        };
        let DestinationSlot::Live { record, .. } =
            std::mem::replace(&mut self.destinations[key.slot() as usize], next)
        else {
            return Err(StructuralValueError::InvariantViolation);
        };
        Ok(record)
    }
}
