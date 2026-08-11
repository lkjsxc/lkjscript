use crate::verify::*;
use crate::{BlockId, Function, IrError, Terminator};

#[derive(Debug)]
pub(crate) struct ControlFlowGraph {
    successors: Vec<Vec<BlockId>>,
    predecessors: Vec<Vec<BlockId>>,
    reachable: Vec<bool>,
    dominators: Dominators,
}

#[derive(Debug)]
pub(crate) struct Dominators {
    tree_enter: Vec<usize>,
    tree_exit: Vec<usize>,
}

#[derive(Clone, Copy)]
struct DfsFrame {
    node: usize,
    next_edge: usize,
}

impl ControlFlowGraph {
    pub(crate) fn build(function: &Function) -> crate::Result<Self> {
        let block_count = function.blocks.len();
        let mut successors = empty_adjacency(block_count, "SSA successor index allocation failed")?;
        let mut predecessors =
            empty_adjacency(block_count, "SSA predecessor index allocation failed")?;

        for block in &function.blocks {
            let block_index = block
                .id
                .index()
                .ok_or_else(|| IrError::new("SSA BlockId cannot index CFG metadata"))?;
            let targets = terminator_successors(&block.terminator);
            let target_count = targets.iter().flatten().count();
            let block_successors = successors
                .get_mut(block_index)
                .ok_or_else(|| IrError::new("SSA successor metadata is inconsistent"))?;
            block_successors
                .try_reserve_exact(target_count)
                .map_err(|_| IrError::new("SSA successor index allocation failed"))?;
            for target in targets.into_iter().flatten() {
                let target_index = target
                    .index()
                    .filter(|index| *index < block_count)
                    .ok_or_else(|| IrError::new("SSA terminator references a missing block"))?;
                block_successors.push(target);
                let target_predecessors = predecessors
                    .get_mut(target_index)
                    .ok_or_else(|| IrError::new("SSA predecessor metadata is inconsistent"))?;
                target_predecessors
                    .try_reserve(1)
                    .map_err(|_| IrError::new("SSA predecessor index allocation failed"))?;
                target_predecessors.push(block.id);
            }
            block_successors.sort_unstable();
            block_successors.dedup();
        }
        for incoming in &mut predecessors {
            incoming.sort_unstable();
            incoming.dedup();
        }

        let reachable = reachable_blocks(function, &successors)?;
        let (components, component_sizes) =
            strongly_connected_components(&successors, &predecessors)?;
        let roots = dominance_roots(
            function,
            &successors,
            &reachable,
            &components,
            &component_sizes,
        )?;
        let dominators = Dominators::compute(&successors, &predecessors, &reachable, &roots)?;
        Ok(Self {
            successors,
            predecessors,
            reachable,
            dominators,
        })
    }

    pub(crate) fn successors(&self, block: BlockId) -> crate::Result<&[BlockId]> {
        block
            .index()
            .and_then(|index| self.successors.get(index))
            .map(Vec::as_slice)
            .ok_or_else(|| IrError::new("SSA successor metadata is inconsistent"))
    }

    pub(crate) fn predecessors(&self, block: BlockId) -> crate::Result<&[BlockId]> {
        block
            .index()
            .and_then(|index| self.predecessors.get(index))
            .map(Vec::as_slice)
            .ok_or_else(|| IrError::new("SSA predecessor metadata is inconsistent"))
    }

    pub(crate) fn is_reachable(&self, block: BlockId) -> crate::Result<bool> {
        block
            .index()
            .and_then(|index| self.reachable.get(index))
            .copied()
            .ok_or_else(|| IrError::new("SSA reachability metadata is inconsistent"))
    }

    pub(crate) fn dominators(&self) -> &Dominators {
        &self.dominators
    }
}

impl Dominators {
    fn compute(
        successors: &[Vec<BlockId>],
        predecessors: &[Vec<BlockId>],
        reachable: &[bool],
        roots: &[usize],
    ) -> crate::Result<Self> {
        let block_count = successors.len();
        let virtual_root = block_count;
        let node_count = block_count
            .checked_add(1)
            .ok_or_else(|| IrError::new("SSA dominator node count overflow"))?;
        let root_flags = bool_vector(block_count, false, "SSA dominator root allocation failed")?;
        let mut root_flags = root_flags;
        for root in roots {
            let flag = root_flags
                .get_mut(*root)
                .ok_or_else(|| IrError::new("SSA dominator root is invalid"))?;
            *flag = true;
        }

        let mut visited = bool_vector(node_count, false, "SSA dominator DFS allocation failed")?;
        let mut postorder = reserved_vec(node_count, "SSA dominator RPO allocation failed")?;
        let mut stack = reserved_vec(node_count, "SSA dominator DFS stack allocation failed")?;
        visited[virtual_root] = true;
        stack.push(DfsFrame {
            node: virtual_root,
            next_edge: 0,
        });
        while let Some(frame) = stack.last_mut() {
            let candidate = if frame.node == virtual_root {
                let candidate = roots.get(frame.next_edge).copied();
                frame.next_edge = frame
                    .next_edge
                    .checked_add(1)
                    .ok_or_else(|| IrError::new("SSA dominator DFS edge index overflow"))?;
                candidate
            } else {
                let edges = successors.get(frame.node).ok_or_else(|| {
                    IrError::new("SSA dominator successor metadata is inconsistent")
                })?;
                let mut candidate = None;
                while let Some(edge) = edges.get(frame.next_edge) {
                    frame.next_edge = frame
                        .next_edge
                        .checked_add(1)
                        .ok_or_else(|| IrError::new("SSA dominator DFS edge index overflow"))?;
                    let edge_index = edge
                        .index()
                        .ok_or_else(|| IrError::new("SSA successor BlockId cannot be indexed"))?;
                    if reachable.get(frame.node) == reachable.get(edge_index) {
                        candidate = Some(edge_index);
                        break;
                    }
                }
                candidate
            };
            if let Some(candidate) = candidate {
                let seen = visited
                    .get_mut(candidate)
                    .ok_or_else(|| IrError::new("SSA dominator DFS target is invalid"))?;
                if !*seen {
                    *seen = true;
                    stack
                        .try_reserve(1)
                        .map_err(|_| IrError::new("SSA dominator DFS stack allocation failed"))?;
                    stack.push(DfsFrame {
                        node: candidate,
                        next_edge: 0,
                    });
                }
            } else {
                let completed = stack
                    .pop()
                    .ok_or_else(|| IrError::new("SSA dominator DFS stack is inconsistent"))?;
                postorder.push(completed.node);
            }
        }
        if visited.iter().any(|seen| !seen) {
            return fail("SSA dominator roots do not cover the CFG");
        }
        postorder.reverse();

        let mut rpo_position = usize_vector(
            node_count,
            usize::MAX,
            "SSA dominator RPO index allocation failed",
        )?;
        for (position, node) in postorder.iter().copied().enumerate() {
            let slot = rpo_position
                .get_mut(node)
                .ok_or_else(|| IrError::new("SSA dominator RPO node is invalid"))?;
            *slot = position;
        }
        let mut immediate =
            option_usize_vector(node_count, "SSA immediate-dominator allocation failed")?;
        immediate[virtual_root] = Some(virtual_root);

        let mut changed = true;
        while changed {
            changed = false;
            for node in postorder.iter().copied().skip(1) {
                let mut next = root_flags
                    .get(node)
                    .copied()
                    .unwrap_or(false)
                    .then_some(virtual_root);
                let incoming = predecessors.get(node).ok_or_else(|| {
                    IrError::new("SSA immediate-dominator predecessor metadata is inconsistent")
                })?;
                for predecessor in incoming {
                    let predecessor = predecessor
                        .index()
                        .ok_or_else(|| IrError::new("SSA predecessor BlockId cannot be indexed"))?;
                    if reachable.get(node) != reachable.get(predecessor)
                        || immediate.get(predecessor).and_then(|item| *item).is_none()
                    {
                        continue;
                    }
                    next = Some(match next {
                        Some(current) => {
                            intersect(current, predecessor, &immediate, &rpo_position)?
                        }
                        None => predecessor,
                    });
                }
                let next = next.ok_or_else(|| {
                    IrError::new("SSA immediate-dominator construction lost a CFG root")
                })?;
                let slot = immediate
                    .get_mut(node)
                    .ok_or_else(|| IrError::new("SSA immediate-dominator node is invalid"))?;
                if *slot != Some(next) {
                    *slot = Some(next);
                    changed = true;
                }
            }
        }

        let mut children =
            empty_usize_adjacency(node_count, "SSA dominator-tree allocation failed")?;
        for node in 0..block_count {
            let parent = immediate
                .get(node)
                .and_then(|item| *item)
                .ok_or_else(|| IrError::new("SSA immediate-dominator metadata is incomplete"))?;
            let siblings = children
                .get_mut(parent)
                .ok_or_else(|| IrError::new("SSA dominator-tree parent is invalid"))?;
            siblings
                .try_reserve(1)
                .map_err(|_| IrError::new("SSA dominator-tree allocation failed"))?;
            siblings.push(node);
        }
        for siblings in &mut children {
            siblings.sort_unstable();
        }

        let mut tree_enter = usize_vector(
            block_count,
            usize::MAX,
            "SSA dominator interval allocation failed",
        )?;
        let mut tree_exit = usize_vector(
            block_count,
            usize::MAX,
            "SSA dominator interval allocation failed",
        )?;
        let interval_capacity = node_count
            .checked_mul(2)
            .ok_or_else(|| IrError::new("SSA dominator traversal size overflow"))?;
        let mut traversal = reserved_vec(
            interval_capacity,
            "SSA dominator traversal allocation failed",
        )?;
        traversal.push((virtual_root, false));
        let mut clock = 0usize;
        while let Some((node, exiting)) = traversal.pop() {
            if exiting {
                if node != virtual_root {
                    tree_exit[node] = clock;
                }
                clock = clock
                    .checked_add(1)
                    .ok_or_else(|| IrError::new("SSA dominator interval overflow"))?;
                continue;
            }
            if node != virtual_root {
                tree_enter[node] = clock;
            }
            clock = clock
                .checked_add(1)
                .ok_or_else(|| IrError::new("SSA dominator interval overflow"))?;
            traversal.push((node, true));
            for child in children[node].iter().rev() {
                traversal.push((*child, false));
            }
        }
        Ok(Self {
            tree_enter,
            tree_exit,
        })
    }
}

pub(crate) fn dominates(
    dominators: &Dominators,
    block: BlockId,
    candidate: BlockId,
) -> crate::Result<bool> {
    let block_index = block
        .index()
        .ok_or_else(|| IrError::new("SSA use BlockId cannot be indexed"))?;
    let candidate_index = candidate
        .index()
        .ok_or_else(|| IrError::new("SSA definition BlockId cannot be indexed"))?;
    let block_enter = dominators
        .tree_enter
        .get(block_index)
        .copied()
        .ok_or_else(|| IrError::new("SSA dominance metadata is inconsistent"))?;
    let block_exit = dominators
        .tree_exit
        .get(block_index)
        .copied()
        .ok_or_else(|| IrError::new("SSA dominance metadata is inconsistent"))?;
    let candidate_enter = dominators
        .tree_enter
        .get(candidate_index)
        .copied()
        .ok_or_else(|| IrError::new("SSA dominance metadata is inconsistent"))?;
    let candidate_exit = dominators
        .tree_exit
        .get(candidate_index)
        .copied()
        .ok_or_else(|| IrError::new("SSA dominance metadata is inconsistent"))?;
    Ok(candidate_enter <= block_enter && block_exit <= candidate_exit)
}

pub(crate) fn verify_loops(function: &Function, cfg: &ControlFlowGraph) -> crate::Result<()> {
    let mut headers = bool_vector(
        function.blocks.len(),
        false,
        "SSA loop-header index allocation failed",
    )?;
    for block in &function.blocks {
        if !cfg.is_reachable(block.id)? {
            continue;
        }
        for successor in cfg.successors(block.id)? {
            if dominates(cfg.dominators(), block.id, *successor)? {
                let target = block_by_id(function, *successor)?;
                if !target.metadata.loop_header {
                    return fail(format!(
                        "SSA backedge targets unmarked loop header {}",
                        successor.raw()
                    ));
                }
                let index = successor
                    .index()
                    .ok_or_else(|| IrError::new("SSA loop-header BlockId cannot be indexed"))?;
                headers[index] = true;
            }
        }
    }
    for block in &function.blocks {
        let index = block
            .id
            .index()
            .ok_or_else(|| IrError::new("SSA loop-header BlockId cannot be indexed"))?;
        if block.metadata.loop_header && !headers[index] {
            return fail(format!(
                "SSA block {} is marked loop-header without a backedge",
                block.id.raw()
            ));
        }
        if block.metadata.loop_header && block.metadata.frame_state.is_none() {
            return fail(format!(
                "SSA loop header {} has no frame state",
                block.id.raw()
            ));
        }
    }
    Ok(())
}

fn terminator_successors(terminator: &Terminator) -> [Option<BlockId>; 2] {
    match terminator {
        Terminator::Branch { target, .. } => [Some(*target), None],
        Terminator::ConditionalBranch {
            true_target,
            false_target,
            ..
        } => [Some(*true_target), Some(*false_target)],
        _ => [None, None],
    }
}

fn reachable_blocks(function: &Function, successors: &[Vec<BlockId>]) -> crate::Result<Vec<bool>> {
    let mut reachable = bool_vector(
        function.blocks.len(),
        false,
        "SSA reachability allocation failed",
    )?;
    let entry = function
        .entry
        .index()
        .ok_or_else(|| IrError::new("SSA entry BlockId cannot be indexed"))?;
    let mut work = reserved_vec(
        function.blocks.len(),
        "SSA reachability worklist allocation failed",
    )?;
    reachable[entry] = true;
    work.push(entry);
    while let Some(current) = work.pop() {
        for successor in successors
            .get(current)
            .ok_or_else(|| IrError::new("SSA reachability metadata is inconsistent"))?
        {
            let successor = successor
                .index()
                .ok_or_else(|| IrError::new("SSA successor BlockId cannot be indexed"))?;
            if !reachable[successor] {
                reachable[successor] = true;
                work.push(successor);
            }
        }
    }
    Ok(reachable)
}

fn strongly_connected_components(
    successors: &[Vec<BlockId>],
    predecessors: &[Vec<BlockId>],
) -> crate::Result<(Vec<usize>, Vec<usize>)> {
    let block_count = successors.len();
    let mut visited = bool_vector(block_count, false, "SSA SCC visited allocation failed")?;
    let mut order = reserved_vec(block_count, "SSA SCC order allocation failed")?;
    let mut stack = reserved_vec(block_count, "SSA SCC DFS stack allocation failed")?;
    for start in 0..block_count {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        stack.push(DfsFrame {
            node: start,
            next_edge: 0,
        });
        while let Some(frame) = stack.last_mut() {
            let edges = successors
                .get(frame.node)
                .ok_or_else(|| IrError::new("SSA SCC successor metadata is inconsistent"))?;
            if let Some(edge) = edges.get(frame.next_edge) {
                frame.next_edge = frame
                    .next_edge
                    .checked_add(1)
                    .ok_or_else(|| IrError::new("SSA SCC edge index overflow"))?;
                let edge = edge
                    .index()
                    .ok_or_else(|| IrError::new("SSA SCC successor cannot be indexed"))?;
                if !visited[edge] {
                    visited[edge] = true;
                    stack.push(DfsFrame {
                        node: edge,
                        next_edge: 0,
                    });
                }
            } else {
                let completed = stack
                    .pop()
                    .ok_or_else(|| IrError::new("SSA SCC DFS stack is inconsistent"))?;
                order.push(completed.node);
            }
        }
    }

    let mut components = usize_vector(
        block_count,
        usize::MAX,
        "SSA SCC component allocation failed",
    )?;
    let mut component_sizes = Vec::new();
    component_sizes
        .try_reserve(block_count)
        .map_err(|_| IrError::new("SSA SCC size allocation failed"))?;
    let mut work = reserved_vec(block_count, "SSA SCC reverse worklist allocation failed")?;
    for start in order.into_iter().rev() {
        if components[start] != usize::MAX {
            continue;
        }
        let component = component_sizes.len();
        let mut size = 0usize;
        components[start] = component;
        work.push(start);
        while let Some(current) = work.pop() {
            size = size
                .checked_add(1)
                .ok_or_else(|| IrError::new("SSA SCC size overflow"))?;
            for predecessor in predecessors
                .get(current)
                .ok_or_else(|| IrError::new("SSA SCC predecessor metadata is inconsistent"))?
            {
                let predecessor = predecessor
                    .index()
                    .ok_or_else(|| IrError::new("SSA SCC predecessor cannot be indexed"))?;
                if components[predecessor] == usize::MAX {
                    components[predecessor] = component;
                    work.push(predecessor);
                }
            }
        }
        component_sizes.push(size);
    }
    Ok((components, component_sizes))
}

fn dominance_roots(
    function: &Function,
    successors: &[Vec<BlockId>],
    reachable: &[bool],
    components: &[usize],
    component_sizes: &[usize],
) -> crate::Result<Vec<usize>> {
    let mut component_is_source = bool_vector(
        component_sizes.len(),
        true,
        "SSA dominator component allocation failed",
    )?;
    let mut component_minimum = usize_vector(
        component_sizes.len(),
        usize::MAX,
        "SSA dominator component allocation failed",
    )?;
    for block in 0..successors.len() {
        if reachable[block] {
            continue;
        }
        let component = components[block];
        component_minimum[component] = component_minimum[component].min(block);
        for successor in &successors[block] {
            let successor = successor
                .index()
                .ok_or_else(|| IrError::new("SSA dominator successor cannot be indexed"))?;
            if reachable[successor] {
                continue;
            }
            let successor_component = components[successor];
            if successor_component != component {
                component_is_source[successor_component] = false;
            }
        }
    }
    let mut roots = reserved_vec(
        component_sizes
            .len()
            .checked_add(1)
            .ok_or_else(|| IrError::new("SSA dominator root count overflow"))?,
        "SSA dominator root allocation failed",
    )?;
    roots.push(
        function
            .entry
            .index()
            .ok_or_else(|| IrError::new("SSA entry BlockId cannot be indexed"))?,
    );
    for component in 0..component_sizes.len() {
        if component_is_source[component] && component_minimum[component] != usize::MAX {
            roots.push(component_minimum[component]);
        }
    }
    roots[1..].sort_unstable();
    Ok(roots)
}

fn intersect(
    mut left: usize,
    mut right: usize,
    immediate: &[Option<usize>],
    rpo_position: &[usize],
) -> crate::Result<usize> {
    while left != right {
        while rpo_position[left] > rpo_position[right] {
            left = immediate[left]
                .ok_or_else(|| IrError::new("SSA immediate-dominator chain is incomplete"))?;
        }
        while rpo_position[right] > rpo_position[left] {
            right = immediate[right]
                .ok_or_else(|| IrError::new("SSA immediate-dominator chain is incomplete"))?;
        }
    }
    Ok(left)
}

fn empty_adjacency(
    count: usize,
    allocation_error: &'static str,
) -> crate::Result<Vec<Vec<BlockId>>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| IrError::new(allocation_error))?;
    output.resize_with(count, Vec::new);
    Ok(output)
}

fn empty_usize_adjacency(
    count: usize,
    allocation_error: &'static str,
) -> crate::Result<Vec<Vec<usize>>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| IrError::new(allocation_error))?;
    output.resize_with(count, Vec::new);
    Ok(output)
}

fn bool_vector(
    count: usize,
    value: bool,
    allocation_error: &'static str,
) -> crate::Result<Vec<bool>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| IrError::new(allocation_error))?;
    output.resize(count, value);
    Ok(output)
}

fn usize_vector(
    count: usize,
    value: usize,
    allocation_error: &'static str,
) -> crate::Result<Vec<usize>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| IrError::new(allocation_error))?;
    output.resize(count, value);
    Ok(output)
}

fn option_usize_vector(
    count: usize,
    allocation_error: &'static str,
) -> crate::Result<Vec<Option<usize>>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| IrError::new(allocation_error))?;
    output.resize(count, None);
    Ok(output)
}

fn reserved_vec<T>(count: usize, allocation_error: &'static str) -> crate::Result<Vec<T>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| IrError::new(allocation_error))?;
    Ok(output)
}
