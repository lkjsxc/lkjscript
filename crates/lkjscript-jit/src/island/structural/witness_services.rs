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
