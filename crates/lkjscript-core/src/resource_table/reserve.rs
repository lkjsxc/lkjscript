use lkjscript_contracts::ResourceKind;

use super::slot::{Binding, OpenSlot, ParentRef, ReservedSlot, SlotState};
use super::{
    BorrowedReservation, OwnedReservation, ProviderId, ResourceKey, ResourceOwnership,
    ResourceTable, ResourceTableError, ResourceTableLimit,
};

impl<P> ResourceTable<P> {
    pub fn reserve_owned(
        &mut self,
        kind: ResourceKind,
        provider: ProviderId,
    ) -> Result<OwnedReservation<'_, P>, ResourceTableError> {
        let (slot, generation, binding, sequence) =
            self.reserve(kind, provider, ResourceOwnership::Owned, None)?;
        Ok(OwnedReservation {
            table: self,
            slot,
            generation,
            binding,
            sequence,
            active: true,
        })
    }

    pub fn reserve_owned_child(
        &mut self,
        parent: &ResourceKey,
        parent_kind: ResourceKind,
        child_kind: ResourceKind,
        provider: ProviderId,
    ) -> Result<OwnedReservation<'_, P>, ResourceTableError> {
        let parent_index =
            self.resolve(parent, parent_kind, provider, self.scope, parent.ownership)?;
        let children = match &self.slots[parent_index].state {
            SlotState::OwnedOpen(open) | SlotState::BorrowedOpen(open) => open.live_children,
            state => {
                return Err(ResourceTableError::InvalidState {
                    state: state.into(),
                })
            }
        };
        if let Some(configured) = self
            .limits
            .max_children_per_parent()
            .filter(|configured| children >= *configured)
        {
            return Err(ResourceTableError::LimitExceeded {
                limit: ResourceTableLimit::ChildrenPerParent,
                configured,
                observed: children
                    .checked_add(1)
                    .ok_or(ResourceTableError::AcquisitionSequenceExhausted)?,
            });
        }
        let parent_ref = ParentRef {
            slot: parent_index,
            generation: parent.generation,
        };
        let (slot, generation, binding, sequence) = self.reserve(
            child_kind,
            provider,
            ResourceOwnership::Owned,
            Some(parent_ref),
        )?;
        match &mut self.slots[parent_index].state {
            SlotState::OwnedOpen(open) | SlotState::BorrowedOpen(open) => {
                open.live_children += 1;
            }
            _ => {}
        }
        Ok(OwnedReservation {
            table: self,
            slot,
            generation,
            binding,
            sequence,
            active: true,
        })
    }

    pub fn reserve_borrowed(
        &mut self,
        kind: ResourceKind,
        provider: ProviderId,
    ) -> Result<BorrowedReservation<'_, P>, ResourceTableError> {
        let (slot, generation, binding, sequence) =
            self.reserve(kind, provider, ResourceOwnership::Borrowed, None)?;
        Ok(BorrowedReservation {
            table: self,
            slot,
            generation,
            binding,
            sequence,
            active: true,
        })
    }

    fn reserve(
        &mut self,
        kind: ResourceKind,
        provider: ProviderId,
        ownership: ResourceOwnership,
        parent: Option<ParentRef>,
    ) -> Result<(usize, std::num::NonZeroU64, Binding, u64), ResourceTableError> {
        let stats = self.stats();
        Self::check_limit(
            stats.reserved(),
            self.limits.max_reserved(),
            ResourceTableLimit::Reservations,
        )?;
        let sequence = match ownership {
            ResourceOwnership::Owned => self
                .next_sequence
                .checked_add(1)
                .ok_or(ResourceTableError::AcquisitionSequenceExhausted)?,
            ResourceOwnership::Borrowed => 0,
        };
        let pending = match ownership {
            ResourceOwnership::Owned => {
                stats.reserved_owned() + stats.owned_open() + stats.closing()
            }
            ResourceOwnership::Borrowed => stats.reserved_borrowed() + stats.borrowed_open(),
        };
        let (limit, limit_kind) = match ownership {
            ResourceOwnership::Owned => (self.limits.max_owned(), ResourceTableLimit::Owned),
            ResourceOwnership::Borrowed => {
                (self.limits.max_borrowed(), ResourceTableLimit::Borrowed)
            }
        };
        Self::check_limit(pending, limit, limit_kind)?;
        let slot = self.take_reusable_slot()?;
        let generation = self.slots[slot].generation;
        let binding = Binding {
            kind,
            provider,
            scope: self.scope,
            ownership,
            parent,
        };
        self.slots[slot].state = SlotState::Reserved(ReservedSlot { binding });
        Ok((slot, generation, binding, sequence))
    }

    fn check_limit(
        current: usize,
        configured: Option<usize>,
        limit: ResourceTableLimit,
    ) -> Result<(), ResourceTableError> {
        let Some(configured) = configured else {
            return Ok(());
        };
        if current < configured {
            Ok(())
        } else {
            Err(ResourceTableError::LimitExceeded {
                limit,
                configured,
                observed: current
                    .checked_add(1)
                    .ok_or(ResourceTableError::AcquisitionSequenceExhausted)?,
            })
        }
    }

    pub(super) fn commit_reservation(
        &mut self,
        slot: usize,
        generation: std::num::NonZeroU64,
        binding: Binding,
        sequence: u64,
        payload: P,
    ) -> ResourceKey {
        self.slots[slot].state = match binding.ownership {
            ResourceOwnership::Owned => {
                self.next_sequence = sequence;
                self.owned_order.push(slot);
                SlotState::OwnedOpen(OpenSlot {
                    binding,
                    payload,
                    sequence,
                    live_children: 0,
                })
            }
            ResourceOwnership::Borrowed => SlotState::BorrowedOpen(OpenSlot {
                binding,
                payload,
                sequence,
                live_children: 0,
            }),
        };
        ResourceKey::new(
            slot,
            generation,
            binding.kind,
            binding.provider,
            binding.scope,
            binding.ownership,
        )
    }

    pub(super) fn cancel_reservation(&mut self, slot: usize, binding: Binding) {
        self.slots[slot].state = SlotState::Vacant;
        self.reusable.push(slot);
        self.release_parent(binding.parent);
    }
}
