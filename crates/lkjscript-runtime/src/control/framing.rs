use super::{ControlError, MAX_CONTROL_FRAME_BYTES};

pub(super) fn frame(body: Vec<u8>) -> Result<Vec<u8>, ControlError> {
    if body.len() > MAX_CONTROL_FRAME_BYTES {
        return Err(ControlError::Oversized);
    }
    let mut frame = Vec::with_capacity(body.len() + 4);
    frame.extend_from_slice(
        &u32::try_from(body.len())
            .map_err(|_| ControlError::Oversized)?
            .to_le_bytes(),
    );
    frame.extend_from_slice(&body);
    Ok(frame)
}

pub(super) fn body(frame: &[u8]) -> Result<&[u8], ControlError> {
    if frame.len() < 4 {
        return Err(ControlError::Malformed("missing length"));
    }
    let length = u32::from_le_bytes(array(&frame[..4])?) as usize;
    if length > MAX_CONTROL_FRAME_BYTES {
        return Err(ControlError::Oversized);
    }
    if frame.len() != length + 4 {
        return Err(ControlError::Malformed("frame length"));
    }
    Ok(&frame[4..])
}

pub(super) fn array<const N: usize>(bytes: &[u8]) -> Result<[u8; N], ControlError> {
    bytes
        .try_into()
        .map_err(|_| ControlError::Malformed("integer width"))
}
