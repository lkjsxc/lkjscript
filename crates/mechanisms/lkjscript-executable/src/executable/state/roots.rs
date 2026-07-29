use super::*;

impl NativeCallState<'_> {
    pub(in crate::executable) fn materialize_frame_roots(
        &mut self,
        frame_index: usize,
    ) -> Result<(), MaterializeRootError> {
        let Some(frame) = self.active_frames.get(frame_index).copied() else {
            return Err(MaterializeRootError::InvalidFrame);
        };
        if frame.rbp.is_null() || frame.safepoint == INVALID_SAFEPOINT {
            return Err(MaterializeRootError::InvalidFrame);
        }
        let Some(entry) = self.image.entries().get(frame.function_ordinal as usize) else {
            return Err(MaterializeRootError::InvalidFrame);
        };
        let Some(frame_facts) = self
            .image
            .frames()
            .iter()
            .find(|facts| facts.function() == entry.function())
        else {
            return Err(MaterializeRootError::InvalidFrame);
        };
        let Some(safepoint) = self.image.safepoints().get(frame.safepoint as usize) else {
            return Err(MaterializeRootError::InvalidFrame);
        };
        if safepoint.id() != frame.safepoint || safepoint.function() != entry.function() {
            return Err(MaterializeRootError::InvalidFrame);
        }
        let additional = safepoint.stack_map().roots().len();
        let next_root_count = self
            .roots
            .len()
            .checked_add(additional)
            .filter(|count| {
                *count <= MAX_MATERIALIZED_ROOTS && *count <= self.maximum_active_values
            })
            .ok_or(MaterializeRootError::Capacity)?;
        let root_additional = next_root_count
            .checked_sub(self.roots.len())
            .ok_or(MaterializeRootError::Capacity)?;
        let address_additional = next_root_count
            .checked_sub(self.root_addresses.len())
            .ok_or(MaterializeRootError::Capacity)?;
        if (root_additional != 0 && self.roots.try_reserve_exact(root_additional).is_err())
            || (address_additional != 0
                && self
                    .root_addresses
                    .try_reserve_exact(address_additional)
                    .is_err())
        {
            return Err(MaterializeRootError::Capacity);
        }
        for root in safepoint.stack_map().roots() {
            let displacement = root.rbp_displacement();
            let in_frame = displacement <= -16
                && displacement % 8 == 0
                && displacement
                    .checked_neg()
                    .and_then(|value| u32::try_from(value).ok())
                    .is_some_and(|value| value <= frame_facts.frame_bytes())
                && frame_facts.homes().iter().any(|home| {
                    home.kind() == root.kind()
                        && home.rbp_displacement() == displacement
                        && home.value_type() == ValueType::Reference(root.reference_type())
                });
            if !in_frame {
                return Err(MaterializeRootError::InvalidFrame);
            }
            // SAFETY: the retained descriptor bounds the negative displacement
            // within this registered generated frame. Homes are aligned words.
            let address = unsafe { frame.rbp.offset(displacement as isize).cast::<u64>() };
            // SAFETY: address is the validated live word home above.
            let opaque_word = unsafe { address.read() };
            self.roots.push(NativeRoot {
                reference_type: root.reference_type(),
                opaque_word,
            });
            self.root_addresses.push(RootAddress {
                address,
                original_word: opaque_word,
                reference_type: root.reference_type(),
                frame_index,
            });
        }
        Ok(())
    }

    pub(in crate::executable) fn invalidate_active_frame(&mut self) {
        if let Some(reservation) = self.pending_reservation.take() {
            self.reserved_native_stack_bytes = self
                .reserved_native_stack_bytes
                .saturating_sub(reservation.frame_bytes);
            self.active_value_homes = self
                .active_value_homes
                .saturating_sub(reservation.value_homes);
        }
        self.metadata_invalid = true;
        self.status = 5;
    }
}
