use crate::verify::*;
use crate::{MemoryWitnessGroupId, Program};
use std::collections::HashSet;

pub(super) fn verify(program: &Program) -> crate::Result<()> {
    let memory = &program.memory;
    let mut prior = None;
    let mut covered = HashSet::new();
    let groups = memory
        .witness_groups
        .iter()
        .map(|group| {
            if group.id == MemoryWitnessGroupId::new([0; 32])
                || prior.is_some_and(|id| id >= group.id)
            {
                return Err(crate::IrError::new(
                    "SSA memory witness groups require sorted unique nonzero IDs",
                ));
            }
            prior = Some(group.id);
            let members = group
                .members
                .iter()
                .map(|member| {
                    let witness = memory.witness(member.witness).ok_or_else(|| {
                        crate::IrError::new("SSA memory witness group member is missing")
                    })?;
                    if !covered.insert(member.witness)
                        || witness.group != group.id
                        || witness.ordinal != member.ordinal
                        || witness.facts.semantic_type != member.semantic_identity
                    {
                        return Err(crate::IrError::new(
                            "SSA memory witness group partition is inconsistent",
                        ));
                    }
                    Ok(lkjscript_contracts::ExecutableMemoryWitnessGroupMember {
                        id: witness.id.bytes(),
                        ordinal: member.ordinal,
                        semantic_identity: member.semantic_identity,
                        facts: witness.facts.clone(),
                        dependencies: witness.dependencies.clone(),
                    })
                })
                .collect::<crate::Result<Vec<_>>>()?;
            Ok(lkjscript_contracts::ExecutableMemoryWitnessGroup {
                id: group.id.bytes(),
                recursive: group.recursive,
                members,
            })
        })
        .collect::<crate::Result<Vec<_>>>()?;
    if covered.len() != memory.witnesses.len() {
        return fail("SSA memory witness groups have missing or extra members");
    }
    lkjscript_contracts::validate_executable_memory_witness_groups(&groups)
        .map_err(|error| crate::IrError::new(error.to_string()))
}
