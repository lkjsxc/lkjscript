use lkjscript_contracts::{ContractDigest, PLATFORM_REVISION};

use super::{ControlStoreError, MAX_KEY_BYTES, MAX_VALUE_BYTES};

pub(super) const HEADER_BYTES: usize = 60;
pub(super) const CHECKSUM_BYTES: usize = 32;

pub(super) fn header_identity(
    bytes: &[u8],
    magic: &[u8; 8],
    contract: ContractDigest,
) -> Result<(), ControlStoreError> {
    if &bytes[..8] != magic {
        return Err(ControlStoreError::Corrupt("magic"));
    }
    let revision = u64::from_le_bytes(array(&bytes[8..16])?);
    if revision != PLATFORM_REVISION {
        return Err(ControlStoreError::StaleRevision { found: revision });
    }
    if bytes[16..48] != contract.as_bytes() {
        return Err(ControlStoreError::ContractMismatch);
    }
    Ok(())
}

pub(super) fn checksum(bytes: &[u8]) -> Result<(), ControlStoreError> {
    let split = bytes.len() - CHECKSUM_BYTES;
    if lkjscript_contracts::sha256(&bytes[..split]) != bytes[split..] {
        return Err(ControlStoreError::Corrupt("checksum"));
    }
    Ok(())
}

pub(super) fn validate(key: &str, value: &[u8]) -> Result<(), ControlStoreError> {
    if key.is_empty() || key.len() > MAX_KEY_BYTES || value.len() > MAX_VALUE_BYTES {
        return Err(ControlStoreError::Limit("key or value"));
    }
    if !key
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-./".contains(&byte))
    {
        return Err(ControlStoreError::InvalidKey);
    }
    Ok(())
}

pub(super) fn take<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    length: usize,
    end: usize,
) -> Result<&'a [u8], ControlStoreError> {
    let next = offset
        .checked_add(length)
        .ok_or(ControlStoreError::Limit("offset"))?;
    if next > end {
        return Err(ControlStoreError::Corrupt("truncated field"));
    }
    let value = &bytes[*offset..next];
    *offset = next;
    Ok(value)
}

pub(super) fn take_u16(
    bytes: &[u8],
    offset: &mut usize,
    end: usize,
) -> Result<usize, ControlStoreError> {
    Ok(u16::from_le_bytes(array(take(bytes, offset, 2, end)?)?) as usize)
}

pub(super) fn take_u32(
    bytes: &[u8],
    offset: &mut usize,
    end: usize,
) -> Result<usize, ControlStoreError> {
    Ok(u32::from_le_bytes(array(take(bytes, offset, 4, end)?)?) as usize)
}

pub(super) fn put_u16(bytes: &mut Vec<u8>, value: usize) -> Result<(), ControlStoreError> {
    bytes.extend_from_slice(
        &u16::try_from(value)
            .map_err(|_| ControlStoreError::Limit("u16"))?
            .to_le_bytes(),
    );
    Ok(())
}

pub(super) fn put_u32(bytes: &mut Vec<u8>, value: usize) -> Result<(), ControlStoreError> {
    bytes.extend_from_slice(
        &u32::try_from(value)
            .map_err(|_| ControlStoreError::Limit("u32"))?
            .to_le_bytes(),
    );
    Ok(())
}

pub(super) fn array<const N: usize>(bytes: &[u8]) -> Result<[u8; N], ControlStoreError> {
    bytes
        .try_into()
        .map_err(|_| ControlStoreError::Corrupt("integer width"))
}
