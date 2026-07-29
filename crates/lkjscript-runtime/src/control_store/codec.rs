use lkjscript_contracts::{ContractDigest, PLATFORM_REVISION};

use super::wire::{
    array, checksum, header_identity, put_u16, put_u32, take, take_u16, take_u32, validate,
    CHECKSUM_BYTES, HEADER_BYTES,
};
use super::{ControlStoreError, Operation, MAX_RECORD_BYTES};

const RECORD_MAGIC: &[u8; 8] = b"LKJCTRL\0";

pub(super) struct DecodedRecords {
    pub records: Vec<(u64, Operation)>,
    pub truncated: bool,
    pub valid_bytes: usize,
}

pub(super) fn record(
    sequence: u64,
    operation: &Operation,
    contract: ContractDigest,
) -> Result<Vec<u8>, ControlStoreError> {
    let payload = payload(operation)?;
    let mut bytes = Vec::with_capacity(HEADER_BYTES + payload.len() + CHECKSUM_BYTES);
    bytes.extend_from_slice(RECORD_MAGIC);
    bytes.extend_from_slice(&PLATFORM_REVISION.to_le_bytes());
    bytes.extend_from_slice(&contract.as_bytes());
    bytes.extend_from_slice(&sequence.to_le_bytes());
    put_u32(&mut bytes, payload.len())?;
    bytes.extend_from_slice(&payload);
    let checksum = lkjscript_contracts::sha256(&bytes);
    bytes.extend_from_slice(&checksum);
    Ok(bytes)
}

pub(super) fn records(
    bytes: &[u8],
    contract: ContractDigest,
) -> Result<DecodedRecords, ControlStoreError> {
    let mut offset = 0;
    let mut output = Vec::new();
    while offset < bytes.len() {
        if bytes.len() - offset < HEADER_BYTES {
            return Ok(DecodedRecords {
                records: output,
                truncated: true,
                valid_bytes: offset,
            });
        }
        let header = &bytes[offset..offset + HEADER_BYTES];
        header_identity(header, RECORD_MAGIC, contract)?;
        let sequence = u64::from_le_bytes(array(&header[48..56])?);
        let length = u32::from_le_bytes(array(&header[56..60])?) as usize;
        if length > MAX_RECORD_BYTES {
            return Err(ControlStoreError::Limit("record payload"));
        }
        let total = HEADER_BYTES
            .checked_add(length)
            .and_then(|value| value.checked_add(CHECKSUM_BYTES))
            .ok_or(ControlStoreError::Limit("record length"))?;
        if bytes.len() - offset < total {
            return Ok(DecodedRecords {
                records: output,
                truncated: true,
                valid_bytes: offset,
            });
        }
        let frame = &bytes[offset..offset + total];
        checksum(frame)?;
        output.push((
            sequence,
            parse_payload(&frame[HEADER_BYTES..HEADER_BYTES + length])?,
        ));
        offset += total;
    }
    Ok(DecodedRecords {
        records: output,
        truncated: false,
        valid_bytes: offset,
    })
}

fn payload(operation: &Operation) -> Result<Vec<u8>, ControlStoreError> {
    let (key, value) = match operation {
        Operation::Put { key, value } => (key, Some(value.as_slice())),
        Operation::Delete { key } => (key, None),
    };
    validate(key, value.unwrap_or_default())?;
    let mut bytes = vec![u8::from(value.is_none()) + 1];
    put_u16(&mut bytes, key.len())?;
    bytes.extend_from_slice(key.as_bytes());
    if let Some(value) = value {
        put_u32(&mut bytes, value.len())?;
        bytes.extend_from_slice(value);
    }
    Ok(bytes)
}

fn parse_payload(bytes: &[u8]) -> Result<Operation, ControlStoreError> {
    let mut offset = 1;
    let kind = *bytes
        .first()
        .ok_or(ControlStoreError::Corrupt("empty operation"))?;
    let key_length = take_u16(bytes, &mut offset, bytes.len())?;
    let key = take(bytes, &mut offset, key_length, bytes.len())?;
    let key = std::str::from_utf8(key)
        .map_err(|_| ControlStoreError::Corrupt("operation key"))?
        .to_string();
    let operation = match kind {
        1 => {
            let length = take_u32(bytes, &mut offset, bytes.len())?;
            let value = take(bytes, &mut offset, length, bytes.len())?.to_vec();
            validate(&key, &value)?;
            Operation::Put { key, value }
        }
        2 => Operation::Delete { key },
        _ => return Err(ControlStoreError::Corrupt("operation kind")),
    };
    if offset != bytes.len() {
        return Err(ControlStoreError::Corrupt("operation trailing bytes"));
    }
    Ok(operation)
}
