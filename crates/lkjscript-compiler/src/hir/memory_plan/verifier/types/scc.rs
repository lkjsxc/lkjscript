use super::*;

pub(crate) fn verified_components(
    adjacency: &[Vec<usize>],
) -> Result<(Vec<usize>, Vec<bool>, u64)> {
    let mut work = 0_u64;
    let mut order = Vec::new();
    order
        .try_reserve(adjacency.len())
        .map_err(|_| Error::msg("memory verifier SCC order allocation failed"))?;
    let mut seen = vec![false; adjacency.len()];
    for root in 0..adjacency.len() {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut stack = vec![(root, 0_usize)];
        while let Some((node, edge)) = stack.pop() {
            verified_scc_charge(&mut work)?;
            if let Some(next) = adjacency[node].get(edge).copied() {
                let next_edge = edge
                    .checked_add(1)
                    .ok_or_else(|| Error::msg("memory verifier SCC edge index overflow"))?;
                stack.push((node, next_edge));
                if !seen[next] {
                    seen[next] = true;
                    stack.push((next, 0));
                }
            } else {
                order.push(node);
            }
        }
    }
    let mut reverse = Vec::new();
    reverse
        .try_reserve(adjacency.len())
        .map_err(|_| Error::msg("memory verifier reverse graph allocation failed"))?;
    reverse.resize_with(adjacency.len(), Vec::new);
    for (from, targets) in adjacency.iter().enumerate() {
        for target in targets {
            reverse[*target].push(from);
        }
    }
    let mut component = vec![usize::MAX; adjacency.len()];
    let mut sizes = Vec::new();
    sizes
        .try_reserve(adjacency.len())
        .map_err(|_| Error::msg("memory verifier SCC size allocation failed"))?;
    while let Some(root) = order.pop() {
        if component[root] != usize::MAX {
            continue;
        }
        let id = sizes.len();
        let mut size = 0_usize;
        let mut stack = vec![root];
        component[root] = id;
        while let Some(node) = stack.pop() {
            verified_scc_charge(&mut work)?;
            size = size
                .checked_add(1)
                .ok_or_else(|| Error::msg("memory verifier SCC size overflow"))?;
            for next in &reverse[node] {
                if component[*next] == usize::MAX {
                    component[*next] = id;
                    stack.push(*next);
                }
            }
        }
        sizes.push(size);
    }
    let mut recursive: Vec<bool> = sizes.iter().map(|size| *size > 1).collect();
    for (node, targets) in adjacency.iter().enumerate() {
        if targets.contains(&node) {
            recursive[component[node]] = true;
        }
    }
    Ok((component, recursive, work))
}

fn verified_scc_charge(work: &mut u64) -> Result<()> {
    *work = work
        .checked_add(1)
        .ok_or_else(|| Error::msg("memory verifier SCC work overflow"))?;
    Ok(())
}
