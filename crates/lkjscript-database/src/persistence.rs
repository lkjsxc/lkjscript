use std::collections::BTreeMap;

use lkjscript_host::{DurableStorage, HostError};

use crate::database::State;
use crate::types::{NamespacedKey, Operation, Value};
use crate::{DatabaseError, DatabaseResult};

pub(crate) fn append_all(
    storage: &dyn DurableStorage,
    name: &str,
    bytes: &[u8],
) -> DatabaseResult<()> {
    let mut written = 0;
    while written < bytes.len() {
        let count = storage.append(name, &bytes[written..])?;
        if count == 0 || count > bytes.len() - written {
            return Err(DatabaseError::Host(HostError::ShortWrite {
                expected: bytes.len() - written,
                written: count,
            }));
        }
        written += count;
    }
    Ok(())
}

pub(crate) fn apply(index: &mut BTreeMap<NamespacedKey, Value>, operations: Vec<Operation>) {
    for operation in operations {
        match operation {
            Operation::Put(name, value) => {
                index.insert(name, value);
            }
            Operation::Delete(name) => {
                index.remove(&name);
            }
        }
    }
}

pub(crate) fn ready(state: &State) -> DatabaseResult<()> {
    if state.closed {
        Err(DatabaseError::Closed)
    } else if state.needs_reopen {
        Err(DatabaseError::NeedsReopen)
    } else {
        Ok(())
    }
}

pub(crate) fn names(name: &str) -> DatabaseResult<(String, String)> {
    if name.is_empty()
        || name.len() > 100
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(DatabaseError::InvalidDatabaseName);
    }
    Ok((format!("{name}.checkpoint"), format!("{name}.wal")))
}
