use std::num::NonZeroU32;

use super::super::{RootKey, StructuralBorrow, StructuralValueKey};
use super::{
    select, SemanticPayload, StructuralEventKind, StructuralObject, StructuralProjection,
    StructuralType, StructuralValueError, StructuralValueLimit, StructuralValueRuntime,
    StructuralViewKey,
};

#[derive(Debug)]
pub(super) struct ViewRecord {
    pub root: RootKey,
    pub loan: StructuralBorrow,
    pub projection: StructuralProjection,
}

#[derive(Debug)]
pub(super) enum ViewSlot {
    Vacant(NonZeroU32),
    Live {
        generation: NonZeroU32,
        record: ViewRecord,
    },
    Retired,
}

impl StructuralValueRuntime {
    pub fn borrow_projected(
        &mut self,
        key: StructuralValueKey,
        root_type: StructuralType,
        projection: StructuralProjection,
        exclusive: bool,
    ) -> Result<StructuralViewKey, StructuralValueError> {
        if projection.path().as_slice().len() > usize::from(self.limits.max_tree_depth) {
            return Err(StructuralValueError::InvalidFieldPath);
        }
        let root = self.resolve_root(key, root_type)?;
        let object = self.objects.get(root)?;
        let StructuralObject::Owned { value, .. } = object else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        self.require_type(value.value_type, root_type)?;
        let selected = select(value, projection.path())?;
        self.require_type(selected.value_type, projection.expected())?;
        self.validate_projection(selected, &projection)?;
        let loan = if exclusive {
            self.roots.borrow_exclusive(key)?
        } else {
            self.roots.borrow_shared(key)?
        };
        let record = ViewRecord {
            root,
            loan,
            projection,
        };
        let view = match self.allocate_view(record) {
            Ok(view) => view,
            Err(failure) => {
                let (error, record) = *failure;
                self.roots.end_borrow(record.loan)?;
                return Err(error);
            }
        };
        self.metrics.borrows = self.metrics.borrows.saturating_add(1);
        self.metrics.views_created = self.metrics.views_created.saturating_add(1);
        self.metrics.live_views = self.metrics.live_views.saturating_add(1);
        self.metrics.peak_live_views = self.metrics.peak_live_views.max(self.metrics.live_views);
        if matches!(
            self.view(view)?.projection,
            StructuralProjection::Utf8 { .. }
        ) {
            self.record(StructuralEventKind::StringView, view.slot(), 0);
        }
        self.record(
            StructuralEventKind::Borrow,
            view.slot(),
            u64::from(exclusive),
        );
        Ok(view)
    }

    fn validate_projection(
        &self,
        value: &super::SemanticValue,
        projection: &StructuralProjection,
    ) -> Result<(), StructuralValueError> {
        let StructuralProjection::Utf8 { start, end, .. } = projection else {
            return Ok(());
        };
        let SemanticPayload::String(bytes) = &value.payload else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        let text = std::str::from_utf8(bytes).map_err(|_| StructuralValueError::InvalidUtf8)?;
        let start = usize::try_from(*start).map_err(|_| StructuralValueError::InvalidRange)?;
        let end = usize::try_from(*end).map_err(|_| StructuralValueError::InvalidRange)?;
        if start > end
            || end > bytes.len()
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
        {
            return Err(StructuralValueError::InvalidRange);
        }
        Ok(())
    }

    pub fn end_view(&mut self, key: StructuralViewKey) -> Result<(), StructuralValueError> {
        let loan = self.view(key)?.loan;
        self.roots.end_borrow(loan)?;
        self.retire_view(key)?;
        self.metrics.views_ended = self.metrics.views_ended.saturating_add(1);
        self.metrics.live_views = self.metrics.live_views.saturating_sub(1);
        self.record(StructuralEventKind::EndView, key.slot(), 0);
        Ok(())
    }

    pub(super) fn view(&self, key: StructuralViewKey) -> Result<&ViewRecord, StructuralValueError> {
        match self.views.get(key.slot() as usize) {
            Some(ViewSlot::Live { generation, record }) if generation.get() == key.generation() => {
                Ok(record)
            }
            _ => Err(StructuralValueError::StaleView),
        }
    }

    fn allocate_view(
        &mut self,
        record: ViewRecord,
    ) -> Result<StructuralViewKey, Box<(StructuralValueError, ViewRecord)>> {
        if self.free_views.try_reserve(1).is_err() {
            return Err(Box::new((StructuralValueError::AllocationFailed, record)));
        }
        let (slot, generation) = if let Some(slot) = self.free_views.pop() {
            let ViewSlot::Vacant(generation) = self.views[slot as usize] else {
                return Err(Box::new((StructuralValueError::InvariantViolation, record)));
            };
            (slot, generation)
        } else {
            let slot = match u32::try_from(self.views.len()) {
                Ok(slot) if slot < self.limits.max_views => slot,
                _ => {
                    return Err(Box::new((
                        StructuralValueError::LimitExceeded(StructuralValueLimit::Views),
                        record,
                    )));
                }
            };
            if self.views.try_reserve(1).is_err() {
                return Err(Box::new((StructuralValueError::AllocationFailed, record)));
            }
            self.views.push(ViewSlot::Vacant(NonZeroU32::MIN));
            (slot, NonZeroU32::MIN)
        };
        self.views[slot as usize] = ViewSlot::Live { generation, record };
        Ok(StructuralViewKey::new(slot, generation))
    }

    fn retire_view(&mut self, key: StructuralViewKey) -> Result<(), StructuralValueError> {
        self.view(key)?;
        let next = if key.generation() >= self.limits.max_generation {
            ViewSlot::Retired
        } else {
            self.free_views.push(key.slot());
            ViewSlot::Vacant(
                NonZeroU32::new(key.generation() + 1)
                    .ok_or(StructuralValueError::InvariantViolation)?,
            )
        };
        self.views[key.slot() as usize] = next;
        Ok(())
    }
}
