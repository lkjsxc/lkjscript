use lkjscript_contracts::{
    ExecutableMemoryWitnessFacts, MemoryWitnessCodec, MemoryWitnessContention, MemoryWitnessCopy,
    MemoryWitnessDomain, MemoryWitnessDrop, MemoryWitnessEquality, MemoryWitnessListElement,
    MemoryWitnessMode, MemoryWitnessPortability, MemoryWitnessRoot, MemoryWitnessSize,
};
use lkjscript_core::Result;

use super::{MemoryType, MemoryWitnessFacts};

pub(crate) fn executable_facts(facts: &MemoryWitnessFacts) -> Result<ExecutableMemoryWitnessFacts> {
    let mut executable = ExecutableMemoryWitnessFacts {
        semantic_type: super::memory_type_identity(&facts.ty)?,
        semantic_contract: facts.semantic_contract,
        mode: map_mode(facts.mode),
        capabilities: facts.capabilities,
        domain: map_domain(facts.domain),
        root: match facts.root_projection {
            super::MemoryRootProjection::None => MemoryWitnessRoot::None,
            super::MemoryRootProjection::Structural => MemoryWitnessRoot::Structural,
        },
        copy: map_copy(facts.copy_share),
        drop: witness_drop(facts),
        equality: map_equality(facts.equality),
        codec: map_codec(facts.process_codec),
        list_element: map_list_element(facts.list_element),
        size: witness_size(&facts.ty, facts.dynamic_size),
        alignment: witness_alignment(&facts.ty),
        contains_borrow: facts.contains_borrow,
        contains_dynamic_owner: facts.contains_dynamic_owner,
        portability: map_portability(facts.portability),
        contention: map_contention(facts.contention),
        operations: Vec::new(),
    };
    executable.operations = lkjscript_contracts::required_memory_witness_operations(&executable);
    Ok(executable)
}

pub(crate) fn executable_dependencies(facts: &MemoryWitnessFacts) -> Vec<[u8; 32]> {
    facts
        .list
        .iter()
        .map(|list| list.element.as_bytes())
        .collect()
}

include!("mappings.rs");
