use crate::structural::{LocalNodeId, StructuralNodeRecord};

use super::{
    StructuralDestinationKey, StructuralEventKind, StructuralSealResult, StructuralValueError,
    StructuralValueRuntime,
};

impl StructuralValueRuntime {
    pub fn finish_destination_sealed(
        &mut self,
        key: StructuralDestinationKey,
    ) -> Result<StructuralSealResult, StructuralValueError> {
        let (image, facts) = self.complete_destination_image(key)?;
        let copied_bytes = copied_image_bytes(&image)?;
        let sealed = self
            .publish_sealed_image(image, facts, false, copied_bytes)
            .map_err(|failure| failure.0)?;
        self.retire_destination(key)?;
        self.metrics.destinations_completed = self.metrics.destinations_completed.saturating_add(1);
        self.metrics.live_destinations = self.metrics.live_destinations.saturating_sub(1);
        self.record(StructuralEventKind::DestinationComplete, key.slot(), 0);
        Ok(sealed)
    }
}

fn copied_image_bytes(image: &super::StructuralImage) -> Result<u64, StructuralValueError> {
    let nodes = u64::from(image.node_count())
        .checked_mul(std::mem::size_of::<StructuralNodeRecord>() as u64)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    let fields = u64::from(image.field_cell_count())
        .checked_mul(std::mem::size_of::<LocalNodeId>() as u64)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    nodes
        .checked_add(fields)
        .and_then(|bytes| bytes.checked_add(u64::from(image.blob_len())))
        .ok_or(StructuralValueError::ArithmeticOverflow)
}
