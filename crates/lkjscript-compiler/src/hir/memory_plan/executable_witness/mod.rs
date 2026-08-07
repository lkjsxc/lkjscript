use lkjscript_contracts::{
    ExecutableMemoryWitnessFacts, MemoryWitnessContention, MemoryWitnessCopy, MemoryWitnessDomain,
    MemoryWitnessDrop, MemoryWitnessEquality, MemoryWitnessListElement, MemoryWitnessMode,
    MemoryWitnessPortability, MemoryWitnessRoot, MemoryWitnessSize, MemoryWitnessSnapshot,
};
use lkjscript_core::Result;

use super::{MemoryType, MemoryWitnessFacts};

pub(crate) fn executable_facts(facts: &MemoryWitnessFacts) -> Result<ExecutableMemoryWitnessFacts> {
    let mut executable = ExecutableMemoryWitnessFacts {
        semantic_type: lkjscript_contracts::semantic_type_closure_hash(&facts.semantic)
            .map_err(|error| lkjscript_core::Error::msg(error.to_string()))?,
        semantic_contract: facts.semantic_contract,
        semantic: facts.semantic.clone(),
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
        snapshot: map_snapshot(facts.semantic_snapshot),
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

include!("mappings.rs");
