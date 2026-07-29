use std::collections::BTreeMap;

use lkjscript_contracts::{ContractDigest, PLATFORM_REVISION};

use super::wire::{
    array, checksum, header_identity, put_u16, put_u32, take, take_u16, take_u32, validate,
    CHECKSUM_BYTES, HEADER_BYTES,
};
use super::ControlStoreError;

const MAGIC: &[u8; 8] = b"LKJCSNP\0";

pub(super) fn encode(
    sequence: u64,
    facts: &BTreeMap<String, Vec<u8>>,
    contract: ContractDigest,
) -> Result<Vec<u8>, ControlStoreError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&PLATFORM_REVISION.to_le_bytes());
    bytes.extend_from_slice(&contract.as_bytes());
    bytes.extend_from_slice(&sequence.to_le_bytes());
    put_u32(&mut bytes, facts.len())?;
    for (key, value) in facts {
        validate(key, value)?;
        put_u16(&mut bytes, key.len())?;
        bytes.extend_from_slice(key.as_bytes());
        put_u32(&mut bytes, value.len())?;
        bytes.extend_from_slice(value);
    }
    let checksum = lkjscript_contracts::sha256(&bytes);
    bytes.extend_from_slice(&checksum);
    Ok(bytes)
}

pub(super) fn decode(
    bytes: &[u8],
    contract: ContractDigest,
) -> Result<(u64, BTreeMap<String, Vec<u8>>), ControlStoreError> {
    if bytes.len() < HEADER_BYTES + CHECKSUM_BYTES {
        return Err(ControlStoreError::Corrupt("snapshot header"));
    }
    header_identity(&bytes[..HEADER_BYTES], MAGIC, contract)?;
    checksum(bytes)?;
    let sequence = u64::from_le_bytes(array(&bytes[48..56])?);
    let count = u32::from_le_bytes(array(&bytes[56..60])?) as usize;
    let mut offset = HEADER_BYTES;
    let end = bytes.len() - CHECKSUM_BYTES;
    let mut facts = BTreeMap::new();
    for _ in 0..count {
        let key_length = take_u16(bytes, &mut offset, end)?;
        let key = take(bytes, &mut offset, key_length, end)?;
        let value_length = take_u32(bytes, &mut offset, end)?;
        let value = take(bytes, &mut offset, value_length, end)?.to_vec();
        let key = std::str::from_utf8(key)
            .map_err(|_| ControlStoreError::Corrupt("snapshot key"))?
            .to_string();
        validate(&key, &value)?;
        if facts.insert(key, value).is_some() {
            return Err(ControlStoreError::Corrupt("duplicate snapshot key"));
        }
    }
    if offset != end {
        return Err(ControlStoreError::Corrupt("snapshot trailing bytes"));
    }
    Ok((sequence, facts))
}
