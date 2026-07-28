use std::collections::HashSet;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::plan::{
    FunctionId, HeapCallDescriptor, LoanType, ReferenceType, RuntimeCallSlot, Signature,
    SourceFunctionId, SourceOrigin, TrapCode, UniqueType, ValueType,
};

mod access;
mod accounting;
mod construction;
mod constructors;
mod entries;
mod error;
mod execution_domain;
mod frames;
mod heap_sites;
mod integrity;
mod layouts;
mod maps;
#[cfg(test)]
mod tests;
mod tokens;
mod values;

use accounting::*;
pub(crate) use constructors::*;
pub use entries::*;
pub use error::*;
pub use frames::*;
pub use heap_sites::*;
use layouts::*;
pub use maps::*;
pub use tokens::*;
pub use values::*;

#[derive(Debug)]
pub struct InstallableImage {
    bytes: Box<[u8]>,
    static_bytes: Box<[Box<[u8]>]>,
    entries: Box<[EntryMetadata]>,
    relocations: Box<[Relocation]>,
    runtime_calls: Box<[RuntimeCallSlot]>,
    execution_domain: NativeExecutionDomain,
    frames: Box<[FrameFacts]>,
    safepoints: Box<[Safepoint]>,
    root_requirements: Box<[RootMapRequirement]>,
    heap_runtime_sites: Box<[HeapRuntimeSite]>,
    source_map: Box<[SourceMapEntry]>,
    trap_map: Box<[TrapMapEntry]>,
    outcome_map: Box<[OutcomeMapEntry]>,
    accounting: CodeAccounting,
    contracts: ImageContracts,
}

pub(crate) struct ImageParts {
    pub(crate) bytes: Vec<u8>,
    pub(crate) static_bytes: Vec<Box<[u8]>>,
    pub(crate) entries: Vec<EntryMetadata>,
    pub(crate) relocations: Vec<Relocation>,
    pub(crate) runtime_calls: Vec<RuntimeCallSlot>,
    pub(crate) execution_domain: NativeExecutionDomain,
    pub(crate) frames: Vec<FrameFacts>,
    pub(crate) safepoints: Vec<Safepoint>,
    pub(crate) root_requirements: Vec<RootMapRequirement>,
    pub(crate) heap_runtime_sites: Vec<HeapRuntimeSite>,
    pub(crate) source_map: Vec<SourceMapEntry>,
    pub(crate) trap_map: Vec<TrapMapEntry>,
    pub(crate) outcome_map: Vec<OutcomeMapEntry>,
    pub(crate) work_units: u64,
    pub(crate) contracts: ImageContracts,
}
