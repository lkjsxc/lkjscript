use super::group_graph::{validate_dag, validate_local_scc};
use super::*;
use std::collections::{BTreeSet, HashMap, HashSet};

pub fn validate_executable_memory_witness_groups(
    groups: &[ExecutableMemoryWitnessGroup],
) -> Result<(), ExecutableMemoryWitnessGroupError> {
    let mut group_index = HashMap::with_capacity(groups.len());
    let mut member_index = HashMap::new();
    for (index, group) in groups.iter().enumerate() {
        if group.id == [0; 32] || group_index.insert(group.id, index).is_some() {
            return fail("memory witness groups require unique nonzero identities");
        }
        if group.members.is_empty() {
            return fail("memory witness group cannot be empty");
        }
        validate_member_order(group)?;
        validate_local_scc(group)?;
        for member in &group.members {
            let ordinal = usize::try_from(member.ordinal).map_err(|_| {
                ExecutableMemoryWitnessGroupError(
                    "memory witness member ordinal exceeds host usize",
                )
            })?;
            if member_index.insert(member.id, (index, ordinal)).is_some() {
                return fail("memory witness member belongs to multiple groups");
            }
        }
    }
    let mut outgoing = vec![BTreeSet::new(); groups.len()];
    for (group_index_value, group) in groups.iter().enumerate() {
        for member in &group.members {
            validate_member(
                group_index_value,
                group,
                member,
                groups,
                &group_index,
                &member_index,
                &mut outgoing,
            )?;
        }
    }
    validate_dag(&outgoing)?;
    for group in groups {
        if executable_memory_witness_group_id(group.recursive, &group.members) != group.id {
            return fail("memory witness group identity is noncanonical");
        }
        for member in &group.members {
            if executable_memory_witness_member_id(
                group.id,
                member.ordinal,
                member.semantic_identity,
            ) != member.id
            {
                return fail("memory witness member identity is noncanonical");
            }
        }
    }
    Ok(())
}

fn validate_member_order(
    group: &ExecutableMemoryWitnessGroup,
) -> Result<(), ExecutableMemoryWitnessGroupError> {
    let has_local = group
        .members
        .iter()
        .flat_map(|member| &member.dependencies)
        .any(|dependency| {
            matches!(
                dependency.target,
                ExecutableMemoryWitnessTarget::LocalMember(_)
            )
        });
    if group.recursive != has_local || (!group.recursive && group.members.len() != 1) {
        return fail("memory witness recursive group classification is invalid");
    }
    for (index, member) in group.members.iter().enumerate() {
        let expected_ordinal = u64::try_from(index).map_err(|_| {
            ExecutableMemoryWitnessGroupError("memory witness member index exceeds u64")
        })?;
        if member.ordinal != expected_ordinal
            || index > 0 && group.members[index - 1].semantic_identity >= member.semantic_identity
            || member.semantic_identity != member.facts.semantic_type
        {
            return fail("memory witness group member order or semantic identity is invalid");
        }
    }
    Ok(())
}

fn validate_member(
    source_group: usize,
    group: &ExecutableMemoryWitnessGroup,
    member: &ExecutableMemoryWitnessGroupMember,
    groups: &[ExecutableMemoryWitnessGroup],
    group_index: &HashMap<[u8; 32], usize>,
    member_index: &HashMap<[u8; 32], (usize, usize)>,
    outgoing: &mut [BTreeSet<usize>],
) -> Result<(), ExecutableMemoryWitnessGroupError> {
    validate_executable_dependencies(&member.facts.semantic, &member.dependencies).map_err(
        |_| ExecutableMemoryWitnessGroupError("memory witness dependency closure is invalid"),
    )?;
    let requirements = semantic_dependency_requirements(&member.facts.semantic).map_err(|_| {
        ExecutableMemoryWitnessGroupError("memory witness semantic requirements are invalid")
    })?;
    let mut roles = HashSet::new();
    for (dependency, (role, expected)) in member.dependencies.iter().zip(requirements) {
        if !roles.insert(role) {
            return fail("memory witness dependency role is duplicated");
        }
        let target = match dependency.target {
            ExecutableMemoryWitnessTarget::LocalMember(ordinal) => {
                let ordinal = usize::try_from(ordinal).map_err(|_| {
                    ExecutableMemoryWitnessGroupError(
                        "local memory witness ordinal exceeds host usize",
                    )
                })?;
                let target =
                    group
                        .members
                        .get(ordinal)
                        .ok_or(ExecutableMemoryWitnessGroupError(
                            "local memory witness ordinal is invalid",
                        ))?;
                if target.facts.semantic.root != expected {
                    return fail("local memory witness edge type is invalid");
                }
                continue;
            }
            ExecutableMemoryWitnessTarget::ExternalMember { group, member } => {
                let target_group =
                    *group_index
                        .get(&group)
                        .ok_or(ExecutableMemoryWitnessGroupError(
                            "external memory witness group is missing",
                        ))?;
                let &(actual_group, ordinal) =
                    member_index
                        .get(&member)
                        .ok_or(ExecutableMemoryWitnessGroupError(
                            "external memory witness member is missing",
                        ))?;
                if target_group != actual_group {
                    return fail("external memory witness group/member identity is inconsistent");
                }
                outgoing[source_group].insert(target_group);
                &groups[target_group].members[ordinal]
            }
        };
        if target.facts.semantic.root != expected {
            return fail("external memory witness edge type is invalid");
        }
    }
    Ok(())
}

fn fail<T>(message: &'static str) -> Result<T, ExecutableMemoryWitnessGroupError> {
    Err(ExecutableMemoryWitnessGroupError(message))
}
