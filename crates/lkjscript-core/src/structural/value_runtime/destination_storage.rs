use super::destination::{DestinationRecord, DestinationSlot};
use super::{StructuralDestinationKey, StructuralValueError, StructuralValueRuntime};

impl StructuralValueRuntime {
    pub(super) fn destination(
        &self,
        key: StructuralDestinationKey,
    ) -> Result<&DestinationRecord, StructuralValueError> {
        let token = self.destination_token(key)?;
        let index =
            usize::try_from(token.slot).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        match self.destinations.get(index) {
            Some(DestinationSlot::Live {
                generation,
                key: current,
                record,
            }) if *current == key && *generation == token.generation => Ok(record),
            _ => Err(StructuralValueError::StaleDestination),
        }
    }

    pub(super) fn destination_mut(
        &mut self,
        key: StructuralDestinationKey,
    ) -> Result<&mut DestinationRecord, StructuralValueError> {
        let token = self.destination_token(key)?;
        let index =
            usize::try_from(token.slot).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        match self.destinations.get_mut(index) {
            Some(DestinationSlot::Live {
                generation,
                key: current,
                record,
            }) if *current == key && *generation == token.generation => Ok(record),
            _ => Err(StructuralValueError::StaleDestination),
        }
    }
}
