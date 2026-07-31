use super::destination::{DestinationRecord, DestinationSlot};
use super::{StructuralDestinationKey, StructuralValueError, StructuralValueRuntime};

impl StructuralValueRuntime {
    pub(super) fn destination(
        &self,
        key: StructuralDestinationKey,
    ) -> Result<&DestinationRecord, StructuralValueError> {
        match self.destinations.get(key.slot() as usize) {
            Some(DestinationSlot::Live { generation, record })
                if generation.get() == key.generation() =>
            {
                Ok(record)
            }
            _ => Err(StructuralValueError::StaleDestination),
        }
    }

    pub(super) fn destination_mut(
        &mut self,
        key: StructuralDestinationKey,
    ) -> Result<&mut DestinationRecord, StructuralValueError> {
        match self.destinations.get_mut(key.slot() as usize) {
            Some(DestinationSlot::Live { generation, record })
                if generation.get() == key.generation() =>
            {
                Ok(record)
            }
            _ => Err(StructuralValueError::StaleDestination),
        }
    }
}
