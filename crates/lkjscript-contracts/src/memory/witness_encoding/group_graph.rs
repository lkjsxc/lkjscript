use super::*;
use std::collections::BTreeSet;

pub(super) fn validate_local_scc(
    group: &ExecutableMemoryWitnessGroup,
) -> Result<(), ExecutableMemoryWitnessGroupError> {
    if !group.recursive {
        return Ok(());
    }
    let mut forward = vec![Vec::new(); group.members.len()];
    let mut reverse = vec![Vec::new(); group.members.len()];
    for (source, member) in group.members.iter().enumerate() {
        for edge in &member.dependencies {
            if let ExecutableMemoryWitnessTarget::LocalMember(target) = edge.target {
                let target = usize::from(target);
                if target >= group.members.len() {
                    return fail("local memory witness ordinal is invalid");
                }
                forward[source].push(target);
                reverse[target].push(source);
            }
        }
    }
    for adjacency in [&forward, &reverse] {
        let mut seen = vec![false; group.members.len()];
        let mut stack = vec![0usize];
        while let Some(node) = stack.pop() {
            if seen[node] {
                continue;
            }
            seen[node] = true;
            stack.extend(adjacency[node].iter().copied());
        }
        if seen.iter().any(|value| !value) {
            return fail("recursive memory witness group is not one exact SCC");
        }
    }
    Ok(())
}

pub(super) fn validate_dag(
    outgoing: &[BTreeSet<usize>],
) -> Result<(), ExecutableMemoryWitnessGroupError> {
    let mut state = vec![0u8; outgoing.len()];
    for root in 0..outgoing.len() {
        let mut stack = vec![(root, false)];
        while let Some((node, exit)) = stack.pop() {
            if exit {
                state[node] = 2;
                continue;
            }
            if state[node] == 1 {
                return fail("external memory witness group graph is cyclic");
            }
            if state[node] == 2 {
                continue;
            }
            state[node] = 1;
            stack.push((node, true));
            for child in outgoing[node].iter().rev() {
                stack.push((*child, false));
            }
        }
    }
    Ok(())
}

fn fail<T>(message: &'static str) -> Result<T, ExecutableMemoryWitnessGroupError> {
    Err(ExecutableMemoryWitnessGroupError(message))
}
