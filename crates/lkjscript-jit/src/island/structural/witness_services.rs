use super::*;
use crate::island::JitIslandServices;

impl JitIslandServices {
    pub(super) fn acquire_witness_owner(
        &mut self,
        witness: u16,
        key: u64,
    ) -> Result<u64, NativeServiceError> {
        let entry = self
            .witnesses
            .resolve(
                witness,
                lkjscript_core::MemoryWitnessOperation::IndependentOwner,
            )?
            .clone();
        let owner = NativeStructuralOwner::new(entry.value_type(), key);
        self.structural
            .require_owner(owner, Some(entry.storage()))?;
        self.structural
            .copy_owner(owner)
            .map(NativeStructuralOwner::opaque_word)
    }

    pub(super) fn compare_witness_values(
        &mut self,
        witness: u16,
        left: u64,
        right: u64,
    ) -> Result<bool, NativeServiceError> {
        let entry = self
            .witnesses
            .resolve(witness, lkjscript_core::MemoryWitnessOperation::Compare)?
            .clone();
        let left_owner = NativeStructuralOwner::new(entry.value_type(), left);
        let right_owner = NativeStructuralOwner::new(entry.value_type(), right);
        self.structural
            .require_owner(left_owner, Some(entry.storage()))?;
        self.structural
            .require_owner(right_owner, Some(entry.storage()))?;
        let left = StructuralValueKey::from_word(left).ok_or(NativeServiceError::Trap)?;
        let right = StructuralValueKey::from_word(right).ok_or(NativeServiceError::Trap)?;
        self.structural.semantic_owners_equal(left, right)
    }

    pub(super) fn dispose_witness_owner(
        &mut self,
        witness: u16,
        key: u64,
    ) -> Result<(), NativeServiceError> {
        let entry = self
            .witnesses
            .resolve(witness, lkjscript_core::MemoryWitnessOperation::Dispose)?
            .clone();
        let owner = NativeStructuralOwner::new(entry.value_type(), key);
        self.structural
            .require_owner(owner, Some(entry.storage()))?;
        self.structural.drop_owner(owner)
    }
}
