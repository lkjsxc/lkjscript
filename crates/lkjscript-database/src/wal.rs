use std::collections::BTreeMap;

use crate::types::{Key, NamespacedKey, Operation, TenantId, Value};
use crate::{DatabaseError, DatabaseResult, RecoveryReport};

const VERSION: u8 = 1;
const PUT: u8 = 1;
const DELETE: u8 = 2;
const COMMIT: u8 = 3;
const MAX_BODY: usize = 2 + 8 + 2 + 4 + 4 + 128 + 4096 + 1024 * 1024;

pub(crate) struct Replay {
    pub commits: Vec<(u64, Vec<Operation>)>,
    pub max_sequence: u64,
    pub report: RecoveryReport,
}

pub(crate) fn operation_frame(sequence: u64, operation: &Operation) -> Vec<u8> {
    let mut body = vec![VERSION];
    match operation {
        Operation::Put(name, value) => {
            body.push(PUT);
            payload(&mut body, sequence, name, Some(value));
        }
        Operation::Delete(name) => {
            body.push(DELETE);
            payload(&mut body, sequence, name, None);
        }
    }
    frame(body)
}

pub(crate) fn commit_frame(sequence: u64) -> Vec<u8> {
    let mut body = vec![VERSION, COMMIT];
    body.extend_from_slice(&sequence.to_le_bytes());
    frame(body)
}

fn payload(body: &mut Vec<u8>, sequence: u64, name: &NamespacedKey, value: Option<&Value>) {
    body.extend_from_slice(&sequence.to_le_bytes());
    body.extend_from_slice(&(name.tenant.as_bytes().len() as u16).to_le_bytes());
    body.extend_from_slice(&(name.key.as_bytes().len() as u32).to_le_bytes());
    body.extend_from_slice(&(value.map_or(0, |item| item.as_bytes().len()) as u32).to_le_bytes());
    body.extend_from_slice(name.tenant.as_bytes());
    body.extend_from_slice(name.key.as_bytes());
    if let Some(value) = value {
        body.extend_from_slice(value.as_bytes());
    }
}

fn frame(body: Vec<u8>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(body.len() + 8);
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&body);
    bytes.extend_from_slice(&checksum(&body).to_le_bytes());
    bytes
}

pub(crate) fn replay(bytes: &[u8]) -> DatabaseResult<Replay> {
    let mut cursor = 0usize;
    let mut pending: BTreeMap<u64, Vec<Operation>> = BTreeMap::new();
    let mut commits = Vec::new();
    let mut max_sequence = 0u64;
    let mut damaged = false;
    while cursor < bytes.len() {
        if bytes.len() - cursor < 8 {
            damaged = true;
            break;
        }
        let body_len = u32::from_le_bytes(
            bytes[cursor..cursor + 4]
                .try_into()
                .map_err(|_| DatabaseError::CorruptWal)?,
        ) as usize;
        if !(10..=MAX_BODY).contains(&body_len) {
            damaged = true;
            break;
        }
        let end = match cursor.checked_add(8 + body_len) {
            Some(end) if end <= bytes.len() => end,
            _ => {
                damaged = true;
                break;
            }
        };
        let body = &bytes[cursor + 4..cursor + 4 + body_len];
        let stored = u32::from_le_bytes(
            bytes[cursor + 4 + body_len..end]
                .try_into()
                .map_err(|_| DatabaseError::CorruptWal)?,
        );
        if checksum(body) != stored {
            damaged = true;
            break;
        }
        let (sequence, operation) = parse_body(body)?;
        max_sequence = max_sequence.max(sequence);
        match operation {
            Some(operation) => pending.entry(sequence).or_default().push(operation),
            None => commits.push((sequence, pending.remove(&sequence).unwrap_or_default())),
        }
        cursor = end;
    }
    let discarded = pending.len();
    Ok(Replay {
        commits,
        max_sequence,
        report: RecoveryReport {
            damaged_tail_discarded: damaged,
            uncommitted_transactions_discarded: discarded,
        },
    })
}

fn parse_body(body: &[u8]) -> DatabaseResult<(u64, Option<Operation>)> {
    if body[0] != VERSION {
        return Err(DatabaseError::CorruptWal);
    }
    let sequence = u64::from_le_bytes(
        body[2..10]
            .try_into()
            .map_err(|_| DatabaseError::CorruptWal)?,
    );
    if body[1] == COMMIT {
        return if body.len() == 10 {
            Ok((sequence, None))
        } else {
            Err(DatabaseError::CorruptWal)
        };
    }
    if body[1] != PUT && body[1] != DELETE || body.len() < 20 {
        return Err(DatabaseError::CorruptWal);
    }
    let tenant_len = u16::from_le_bytes(
        body[10..12]
            .try_into()
            .map_err(|_| DatabaseError::CorruptWal)?,
    ) as usize;
    let key_len = u32::from_le_bytes(
        body[12..16]
            .try_into()
            .map_err(|_| DatabaseError::CorruptWal)?,
    ) as usize;
    let value_len = u32::from_le_bytes(
        body[16..20]
            .try_into()
            .map_err(|_| DatabaseError::CorruptWal)?,
    ) as usize;
    let expected = 20usize
        .checked_add(tenant_len)
        .and_then(|n| n.checked_add(key_len))
        .and_then(|n| n.checked_add(value_len))
        .ok_or(DatabaseError::CorruptWal)?;
    if expected != body.len() || body[1] == DELETE && value_len != 0 {
        return Err(DatabaseError::CorruptWal);
    }
    let tenant_end = 20 + tenant_len;
    let key_end = tenant_end + key_len;
    let tenant =
        TenantId::new(body[20..tenant_end].to_vec()).map_err(|_| DatabaseError::CorruptWal)?;
    let key =
        Key::new(body[tenant_end..key_end].to_vec()).map_err(|_| DatabaseError::CorruptWal)?;
    let name = NamespacedKey { tenant, key };
    if body[1] == PUT {
        let value = Value::new(body[key_end..].to_vec()).map_err(|_| DatabaseError::CorruptWal)?;
        Ok((sequence, Some(Operation::Put(name, value))))
    } else {
        Ok((sequence, Some(Operation::Delete(name))))
    }
}

pub(crate) fn checksum(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}
