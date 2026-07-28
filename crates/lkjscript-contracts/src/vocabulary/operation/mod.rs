mod bytes;
mod core;
mod host;
mod memory;
mod network;
mod semantics;
mod sqlite;

pub use semantics::{
    operation_semantics_by_id, OperationEffects, OperationOwnership, OperationSemanticsRecord,
    RuntimeLowering, SemanticSourceRelationship,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationIdentity(u16);

impl OperationIdentity {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationCategory {
    Arithmetic,
    Ordering,
    Equality,
    Boolean,
    Bit,
    Conversion,
    List,
    Text,
    ByteData,
    Path,
    Arguments,
    Stdio,
    Resource,
    File,
    Entropy,
    Sqlite,
    Network,
    Terminal,
    Control,
    Variant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationVocabularyRecord {
    pub identity: OperationIdentity,
    pub stable_name: &'static str,
    pub source_name: &'static str,
    pub category: OperationCategory,
    pub summary: &'static str,
    pub semantics: &'static OperationSemanticsRecord,
}

const fn record(
    identity: u16,
    stable_name: &'static str,
    source_name: &'static str,
    category: OperationCategory,
    summary: &'static str,
) -> OperationVocabularyRecord {
    let identity = OperationIdentity::new(identity);
    let semantics = semantics::required_operation_semantics(identity);
    OperationVocabularyRecord {
        identity,
        stable_name,
        source_name,
        category,
        summary,
        semantics,
    }
}

pub const OPERATION_COUNT: usize = 131;

pub fn operation_by_id(identity: OperationIdentity) -> Option<&'static OperationVocabularyRecord> {
    let index = identity.index();
    if index < core::RECORDS.len() {
        return core::RECORDS.get(index);
    }
    let index = index - core::RECORDS.len();
    if index < memory::RECORDS.len() {
        return memory::RECORDS.get(index);
    }
    let index = index - memory::RECORDS.len();
    if index < host::RECORDS.len() {
        return host::RECORDS.get(index);
    }
    let index = index - host::RECORDS.len();
    if index < sqlite::RECORDS.len() {
        return sqlite::RECORDS.get(index);
    }
    let index = index - sqlite::RECORDS.len();
    if index < network::RECORDS.len() {
        return network::RECORDS.get(index);
    }
    bytes::RECORDS.get(index - network::RECORDS.len())
}

pub fn operation_by_source_name(name: &str) -> Option<&'static OperationVocabularyRecord> {
    (0..OPERATION_COUNT).find_map(|index| {
        let record = operation_by_id(OperationIdentity::new(index as u16))?;
        (record.source_name == name).then_some(record)
    })
}
