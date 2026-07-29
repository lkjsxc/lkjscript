use std::collections::BTreeMap;

use crate::types::{Key, NamespacedKey, TenantId, Value};
use crate::{DatabaseError, DatabaseResult};

const MAGIC: &[u8; 8] = b"LKJDBCP1";

pub(crate) fn encode(index: &BTreeMap<NamespacedKey, Value>, sequence: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&sequence.to_le_bytes());
    bytes.extend_from_slice(&(index.len() as u32).to_le_bytes());
    for (name, value) in index {
        bytes.extend_from_slice(&(name.tenant.as_bytes().len() as u16).to_le_bytes());
        bytes.extend_from_slice(&(name.key.as_bytes().len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(value.as_bytes().len() as u32).to_le_bytes());
        bytes.extend_from_slice(name.tenant.as_bytes());
        bytes.extend_from_slice(name.key.as_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    let checksum = crate::wal::checksum(&bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    bytes
}

pub(crate) fn decode(bytes: &[u8]) -> DatabaseResult<(BTreeMap<NamespacedKey, Value>, u64)> {
    if bytes.len() < 24 || bytes.get(..8) != Some(MAGIC) {
        return Err(DatabaseError::CorruptCheckpoint);
    }
    let content_len = bytes.len() - 4;
    let stored = u32::from_le_bytes(
        bytes[content_len..]
            .try_into()
            .map_err(|_| DatabaseError::CorruptCheckpoint)?,
    );
    if crate::wal::checksum(&bytes[..content_len]) != stored {
        return Err(DatabaseError::CorruptCheckpoint);
    }
    let mut cursor = 8;
    let sequence = take_u64(bytes, &mut cursor, content_len)?;
    let count = take_u32(bytes, &mut cursor, content_len)?;
    let mut index = BTreeMap::new();
    for _ in 0..count {
        let tenant_len = usize::from(take_u16(bytes, &mut cursor, content_len)?);
        let key_len = take_u32(bytes, &mut cursor, content_len)? as usize;
        let value_len = take_u32(bytes, &mut cursor, content_len)? as usize;
        let tenant = TenantId::new(take(bytes, &mut cursor, tenant_len, content_len)?.to_vec())
            .map_err(|_| DatabaseError::CorruptCheckpoint)?;
        let key = Key::new(take(bytes, &mut cursor, key_len, content_len)?.to_vec())
            .map_err(|_| DatabaseError::CorruptCheckpoint)?;
        let value = Value::new(take(bytes, &mut cursor, value_len, content_len)?.to_vec())
            .map_err(|_| DatabaseError::CorruptCheckpoint)?;
        if index.insert(NamespacedKey { tenant, key }, value).is_some() {
            return Err(DatabaseError::CorruptCheckpoint);
        }
    }
    if cursor != content_len {
        return Err(DatabaseError::CorruptCheckpoint);
    }
    Ok((index, sequence))
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    length: usize,
    end: usize,
) -> DatabaseResult<&'a [u8]> {
    let next = cursor
        .checked_add(length)
        .ok_or(DatabaseError::CorruptCheckpoint)?;
    if next > end {
        return Err(DatabaseError::CorruptCheckpoint);
    }
    let value = &bytes[*cursor..next];
    *cursor = next;
    Ok(value)
}

fn take_u16(bytes: &[u8], cursor: &mut usize, end: usize) -> DatabaseResult<u16> {
    let value = take(bytes, cursor, 2, end)?;
    Ok(u16::from_le_bytes(
        value
            .try_into()
            .map_err(|_| DatabaseError::CorruptCheckpoint)?,
    ))
}

fn take_u32(bytes: &[u8], cursor: &mut usize, end: usize) -> DatabaseResult<u32> {
    let value = take(bytes, cursor, 4, end)?;
    Ok(u32::from_le_bytes(
        value
            .try_into()
            .map_err(|_| DatabaseError::CorruptCheckpoint)?,
    ))
}

fn take_u64(bytes: &[u8], cursor: &mut usize, end: usize) -> DatabaseResult<u64> {
    let value = take(bytes, cursor, 8, end)?;
    Ok(u64::from_le_bytes(
        value
            .try_into()
            .map_err(|_| DatabaseError::CorruptCheckpoint)?,
    ))
}
