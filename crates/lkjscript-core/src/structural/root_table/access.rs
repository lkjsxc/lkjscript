use super::super::{LayoutIdentity, RootKey, SemanticTypeIdentity};
use super::{
    RootSlot, StructuralRootOwnership, StructuralRootState, StructuralRootTable,
    StructuralRootTableError, StructuralValueKey, TerminalState,
};

impl StructuralRootTable {
    pub fn root(
        &self,
        key: StructuralValueKey,
        layout: LayoutIdentity,
        semantic_type: SemanticTypeIdentity,
    ) -> Result<RootKey, StructuralRootTableError> {
        let index = self.live_root_index(key)?;
        let RootSlot::Live { value, .. } = self.roots[index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        if value.root.layout() != layout {
            return Err(StructuralRootTableError::WrongLayout);
        }
        if value.root.semantic_type() != semantic_type {
            return Err(StructuralRootTableError::WrongSemanticType);
        }
        Ok(value.root)
    }

    pub fn state(
        &self,
        key: StructuralValueKey,
    ) -> Result<StructuralRootState, StructuralRootTableError> {
        let index = usize::try_from(key.slot())
            .map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
        let slot = self
            .roots
            .get(index)
            .ok_or(StructuralRootTableError::StaleRoot)?;
        match *slot {
            RootSlot::Live { generation, value } if generation.get() == key.generation() => {
                if value.exclusive_loan {
                    return Ok(StructuralRootState::BorrowedExclusive);
                }
                if value.shared_loans != 0 {
                    return Ok(StructuralRootState::BorrowedShared);
                }
                Ok(match value.ownership {
                    StructuralRootOwnership::Owned => StructuralRootState::Owned,
                    StructuralRootOwnership::Static => StructuralRootState::Static,
                    StructuralRootOwnership::SealedShared => StructuralRootState::SealedShared,
                })
            }
            RootSlot::Vacant {
                previous: Some((generation, TerminalState::Moved)),
                ..
            } if generation.get() == key.generation() => Ok(StructuralRootState::Moved),
            RootSlot::Vacant {
                previous: Some((generation, TerminalState::Dropped)),
                ..
            } if generation.get() == key.generation() => Ok(StructuralRootState::Dropped),
            RootSlot::Retired { generation } if generation.get() == key.generation() => {
                Ok(StructuralRootState::Retired)
            }
            _ => Err(StructuralRootTableError::StaleRoot),
        }
    }

    pub(super) fn live_root_index(
        &self,
        key: StructuralValueKey,
    ) -> Result<usize, StructuralRootTableError> {
        let index = usize::try_from(key.slot())
            .map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
        match self.roots.get(index) {
            Some(RootSlot::Live { generation, .. }) if generation.get() == key.generation() => {
                Ok(index)
            }
            Some(RootSlot::Vacant {
                previous: Some((generation, TerminalState::Moved)),
                ..
            }) if generation.get() == key.generation() => Err(StructuralRootTableError::MovedRoot),
            Some(RootSlot::Vacant {
                previous: Some((generation, TerminalState::Dropped)),
                ..
            }) if generation.get() == key.generation() => {
                Err(StructuralRootTableError::DroppedRoot)
            }
            Some(RootSlot::Retired { generation }) if generation.get() == key.generation() => {
                Err(StructuralRootTableError::RetiredRoot)
            }
            _ => Err(StructuralRootTableError::StaleRoot),
        }
    }
}
