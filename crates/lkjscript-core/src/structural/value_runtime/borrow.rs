use std::num::NonZeroU64;

use super::super::{RootKey, StructuralBorrow, StructuralValueKey};
use super::{
    LocalNodeId, PrivateTokenKind, StructuralEventKind, StructuralNode, StructuralObject,
    StructuralProjection, StructuralType, StructuralValueError, StructuralValueRuntime,
    StructuralViewKey,
};

#[derive(Debug)]
pub(super) struct ViewRecord {
    pub root: RootKey,
    pub node: LocalNodeId,
    pub loan: StructuralBorrow,
    pub projection: StructuralProjection,
}

#[derive(Debug)]
pub(super) enum ViewSlot {
    Vacant(NonZeroU64),
    Live {
        generation: NonZeroU64,
        key: StructuralViewKey,
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
        let root = self.resolve_root(key, root_type)?;
        let object = self.objects.get(root)?;
        let image = match object {
            StructuralObject::Owned { image, .. } | StructuralObject::Sealed { image, .. } => image,
            StructuralObject::Static(_) => return Err(StructuralValueError::WrongPayloadKind),
        };
        self.require_type(image.root().value_type(), root_type)?;
        let selected = image.selected_node(projection.path())?;
        self.require_type(selected.value_type(), projection.expected())?;
        self.validate_projection(selected, &projection)?;
        let next_allocation = self.next_allocation_event()?;
        let loan = if exclusive {
            self.roots.borrow_exclusive(key)?
        } else {
            self.roots.borrow_shared(key)?
        };
        let record = ViewRecord {
            root,
            node: selected.id(),
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
        self.allocation_events = next_allocation;
        self.metrics.borrows = self.metrics.borrows.saturating_add(1);
        self.metrics.views_created = self.metrics.views_created.saturating_add(1);
        self.metrics.live_views = self.metrics.live_views.saturating_add(1);
        self.metrics.peak_live_views = self.metrics.peak_live_views.max(self.metrics.live_views);
        if matches!(
            self.view(view)?.projection,
            StructuralProjection::Utf8 { .. }
        ) {
            self.record(StructuralEventKind::StringView, view.get(), 0);
        }
        self.record(
            StructuralEventKind::Borrow,
            view.get(),
            u64::from(exclusive),
        );
        Ok(view)
    }

    fn validate_projection(
        &self,
        value: StructuralNode<'_>,
        projection: &StructuralProjection,
    ) -> Result<(), StructuralValueError> {
        let StructuralProjection::Utf8 { start, end, .. } = projection else {
            return Ok(());
        };
        let super::super::image::StructuralNodeView::Bytes(bytes) = value.payload() else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        if value.value_type().kind != super::StructuralKind::String {
            return Err(StructuralValueError::WrongPayloadKind);
        }
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
        self.record(StructuralEventKind::EndView, key.get(), 0);
        Ok(())
    }

    pub(super) fn view(&self, key: StructuralViewKey) -> Result<&ViewRecord, StructuralValueError> {
        let token = self.view_token(key)?;
        let index =
            usize::try_from(token.slot).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        match self.views.get(index) {
            Some(ViewSlot::Live {
                generation,
                key: current,
                record,
            }) if *current == key && *generation == token.generation => Ok(record),
            _ => Err(StructuralValueError::StaleView),
        }
    }

    fn allocate_view(
        &mut self,
        record: ViewRecord,
    ) -> Result<StructuralViewKey, Box<(StructuralValueError, ViewRecord)>> {
        let (slot, generation, reused) = if let Some(&slot) = self.free_views.last() {
            let index = match usize::try_from(slot) {
                Ok(index) => index,
                Err(_) => return Err(Box::new((StructuralValueError::ArithmeticOverflow, record))),
            };
            let Some(ViewSlot::Vacant(generation)) = self.views.get(index) else {
                return Err(Box::new((StructuralValueError::InvariantViolation, record)));
            };
            (slot, *generation, true)
        } else {
            if self.views.try_reserve(1).is_err() {
                return Err(Box::new((StructuralValueError::AllocationFailed, record)));
            }
            let slot = match u64::try_from(self.views.len()) {
                Ok(slot) => slot,
                Err(_) => return Err(Box::new((StructuralValueError::ArithmeticOverflow, record))),
            };
            (slot, NonZeroU64::MIN, false)
        };
        let token = match self.allocate_private_token(PrivateTokenKind::View, slot, generation) {
            Ok(token) => token,
            Err(error) => return Err(Box::new((error, record))),
        };
        let key = StructuralViewKey::from_token(token);
        let index = match usize::try_from(slot) {
            Ok(index) => index,
            Err(_) => return Err(Box::new((StructuralValueError::ArithmeticOverflow, record))),
        };
        if reused {
            if self.free_views.pop() != Some(slot) {
                return Err(Box::new((StructuralValueError::InvariantViolation, record)));
            }
            self.views[index] = ViewSlot::Live {
                generation,
                key,
                record,
            };
        } else {
            self.views.push(ViewSlot::Live {
                generation,
                key,
                record,
            });
        }
        Ok(key)
    }

    fn retire_view(&mut self, key: StructuralViewKey) -> Result<(), StructuralValueError> {
        self.view(key)?;
        let token = self.view_token(key)?;
        let index =
            usize::try_from(token.slot).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let generation = token.generation;
        let next = if generation.get() == u64::MAX {
            ViewSlot::Retired
        } else {
            self.free_views.try_reserve(1)?;
            self.free_views.push(token.slot);
            ViewSlot::Vacant(
                generation
                    .get()
                    .checked_add(1)
                    .and_then(NonZeroU64::new)
                    .ok_or(StructuralValueError::ArithmeticOverflow)?,
            )
        };
        self.views[index] = next;
        self.private_tokens.remove(&key.get());
        Ok(())
    }
}
