#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IdentityEncodingError {
    LengthOverflow,
    AllocationFailure,
}

pub(crate) fn append_framed(
    output: &mut Vec<u8>,
    bytes: &[u8],
) -> Result<(), IdentityEncodingError> {
    let length = u64::try_from(bytes.len()).map_err(|_| IdentityEncodingError::LengthOverflow)?;
    let additional = 8_usize
        .checked_add(bytes.len())
        .ok_or(IdentityEncodingError::LengthOverflow)?;
    output
        .try_reserve(additional)
        .map_err(|_| IdentityEncodingError::AllocationFailure)?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
