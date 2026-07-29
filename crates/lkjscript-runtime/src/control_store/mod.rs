use std::collections::BTreeMap;

use lkjscript_contracts::{current_contracts, ContractDigest, RegisteredContract};
use lkjscript_host::{DurableStorage, HostError};

mod codec;
mod error;
mod snapshot;
#[cfg(test)]
mod tests;
mod wire;

pub use error::ControlStoreError;

const JOURNAL: &str = "control.journal";
const SNAPSHOT: &str = "control.snapshot";
pub const MAX_KEY_BYTES: usize = 128;
pub const MAX_VALUE_BYTES: usize = 65_536;
pub const MAX_RECORD_BYTES: usize = MAX_KEY_BYTES + MAX_VALUE_BYTES + 16;
pub const MAX_FACTS: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryReport {
    pub sequence: u64,
    pub facts: usize,
    pub repaired_truncated_tail: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Operation {
    Put { key: String, value: Vec<u8> },
    Delete { key: String },
}

pub struct ControlStore<S> {
    storage: S,
    contract: ContractDigest,
    sequence: u64,
    facts: BTreeMap<String, Vec<u8>>,
    recovery: RecoveryReport,
}

impl<S: DurableStorage> ControlStore<S> {
    pub fn open(storage: S) -> Result<Self, ControlStoreError> {
        let contract = control_contract()?;
        let (mut sequence, mut facts) = match storage.read(SNAPSHOT)? {
            Some(bytes) if !bytes.is_empty() => snapshot::decode(&bytes, contract)?,
            _ => (0, BTreeMap::new()),
        };
        let journal = storage.read(JOURNAL)?.unwrap_or_default();
        let decoded = codec::records(&journal, contract)?;
        for (record_sequence, operation) in decoded.records {
            if record_sequence <= sequence {
                continue;
            }
            let expected = sequence
                .checked_add(1)
                .ok_or(ControlStoreError::SequenceExhausted)?;
            if record_sequence != expected {
                return Err(ControlStoreError::Corrupt("journal sequence"));
            }
            apply(&mut facts, operation)?;
            sequence = record_sequence;
        }
        if decoded.truncated {
            storage.replace(JOURNAL, &journal[..decoded.valid_bytes])?;
        }
        let recovery = RecoveryReport {
            sequence,
            facts: facts.len(),
            repaired_truncated_tail: decoded.truncated,
        };
        Ok(Self {
            storage,
            contract,
            sequence,
            facts,
            recovery,
        })
    }

    pub fn recovery_report(&self) -> &RecoveryReport {
        &self.recovery
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.facts.get(key).map(Vec::as_slice)
    }

    pub fn facts(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.facts
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_slice()))
    }

    pub fn put(&mut self, key: String, value: Vec<u8>) -> Result<u64, ControlStoreError> {
        self.publish(Operation::Put { key, value })
    }

    pub fn delete(&mut self, key: String) -> Result<u64, ControlStoreError> {
        self.publish(Operation::Delete { key })
    }

    pub fn checkpoint(&mut self) -> Result<(), ControlStoreError> {
        let bytes = snapshot::encode(self.sequence, &self.facts, self.contract)?;
        self.storage.replace(SNAPSHOT, &bytes)?;
        self.storage.replace(JOURNAL, &[])?;
        Ok(())
    }

    fn publish(&mut self, operation: Operation) -> Result<u64, ControlStoreError> {
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or(ControlStoreError::SequenceExhausted)?;
        let frame = codec::record(sequence, &operation, self.contract)?;
        let prior = self.storage.read(JOURNAL)?.unwrap_or_default();
        if let Err(error) =
            append_all(&self.storage, &frame).and_then(|()| self.storage.sync(JOURNAL))
        {
            self.storage.replace(JOURNAL, &prior)?;
            return Err(error.into());
        }
        apply(&mut self.facts, operation)?;
        self.sequence = sequence;
        Ok(sequence)
    }
}

fn control_contract() -> Result<ContractDigest, ControlStoreError> {
    current_contracts()
        .ok()
        .and_then(|contracts| {
            contracts
                .get(lkjscript_contracts::RUNTIME_CONTROL)
                .map(RegisteredContract::digest)
        })
        .ok_or(ControlStoreError::ContractUnavailable)
}

fn append_all(storage: &impl DurableStorage, mut bytes: &[u8]) -> Result<(), HostError> {
    while !bytes.is_empty() {
        let written = storage.append(JOURNAL, bytes)?;
        if written == 0 || written > bytes.len() {
            return Err(HostError::ShortWrite {
                expected: bytes.len(),
                written,
            });
        }
        bytes = &bytes[written..];
    }
    Ok(())
}

fn apply(
    facts: &mut BTreeMap<String, Vec<u8>>,
    operation: Operation,
) -> Result<(), ControlStoreError> {
    match operation {
        Operation::Put { key, value } => {
            if !facts.contains_key(&key) && facts.len() == MAX_FACTS {
                return Err(ControlStoreError::Limit("fact count"));
            }
            facts.insert(key, value);
        }
        Operation::Delete { key } => {
            facts.remove(&key);
        }
    }
    Ok(())
}
