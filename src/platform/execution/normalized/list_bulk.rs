//! Assemble only final immutable nodes; no published prefix needs path copying.
use super::super::value::{NormalizedValue, RawArguments};
use super::{
    Branches, Charge, FANOUT, INDEX_LEVELS, List, Node, admit_length, branch, element, failure,
    invalid, leaf, node,
};
use crate::platform::execution::ExecutionError;
use std::sync::Arc;

struct PrefixFrame {
    children: Branches,
    height: usize,
    remaining: usize,
    next: usize,
}

fn build_leaf(
    pending: &mut RawArguments,
    count: usize,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Arc<Node>, ExecutionError> {
    if count == 0 || count > FANOUT || count > pending.len() {
        return Err(invalid());
    }
    let mut slots = leaf(None, reserve)?;
    for slot in slots.iter_mut().take(count) {
        // element owns a rejected raw item before its fallible reservation.
        *slot = Some(element(pending.next().ok_or_else(invalid)?, reserve)?);
    }
    Ok(node(Node::Leaf(slots)))
}

fn build_prefix(
    pending: &mut RawArguments,
    height: usize,
    leaves: usize,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Arc<Node>, ExecutionError> {
    if leaves == 0 || leaves > List::capacity(height)? {
        return Err(invalid());
    }
    if height == 0 {
        return build_leaf(pending, FANOUT, reserve);
    }
    // Prefix construction settles each branch after its children. Each frame
    // consumes one radix level, so the index-width worklist covers every list
    // extent without adding a heap scratch allocation or a logical depth policy.
    let mut frames: [Option<PrefixFrame>; INDEX_LEVELS] = std::array::from_fn(|_| None);
    frames[0] = Some(PrefixFrame {
        children: branch(None, reserve)?,
        height,
        remaining: leaves,
        next: 0,
    });
    let mut depth = 1;
    loop {
        let frame = frames[depth - 1].as_mut().ok_or_else(invalid)?;
        if frame.remaining == 0 {
            let frame = frames[depth - 1].take().ok_or_else(invalid)?;
            depth -= 1;
            let completed = node(Node::Branch(frame.children));
            if depth == 0 {
                return Ok(completed);
            }
            let parent = frames[depth - 1].as_mut().ok_or_else(invalid)?;
            let slot = parent.next.checked_sub(1).ok_or_else(invalid)?;
            *parent.children.get_mut(slot).ok_or_else(invalid)? = Some(completed);
            continue;
        }
        let child_height = frame.height.checked_sub(1).ok_or_else(invalid)?;
        let span = List::capacity(child_height)?;
        let count = frame.remaining.min(span);
        let slot = frame.next;
        frame.children.get(slot).ok_or_else(invalid)?;
        frame.next += 1;
        frame.remaining -= count;
        if child_height == 0 {
            frame.children[slot] = Some(build_leaf(pending, FANOUT, reserve)?);
        } else {
            *frames.get_mut(depth).ok_or_else(invalid)? = Some(PrefixFrame {
                children: branch(None, reserve)?,
                height: child_height,
                remaining: count,
                next: 0,
            });
            depth += 1;
        }
    }
}

pub(super) fn from_items(
    items: Vec<NormalizedValue>,
    maximum_length: Option<u64>,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<List, ExecutionError> {
    let mut pending = RawArguments::new(items);
    let length = pending.len();
    admit_length(length, maximum_length)?;
    let prefix_leaves = length.saturating_sub(1) / FANOUT;
    let mut result = List {
        length,
        height: 0,
        root: None,
        tail: None,
    };
    if prefix_leaves != 0 {
        while prefix_leaves > List::capacity(result.height)? {
            result.height = result.height.checked_add(1).ok_or_else(failure)?;
        }
        result.root = Some(build_prefix(
            &mut pending,
            result.height,
            prefix_leaves,
            reserve,
        )?);
    }
    let tail_length = length - prefix_leaves * FANOUT;
    if pending.len() != tail_length {
        return Err(invalid());
    }
    if tail_length != 0 {
        result.tail = Some(build_leaf(&mut pending, tail_length, reserve)?);
    }
    // This separate checkpoint also covers the empty list and cancellation
    // after the final payload. No partially assembled List escapes on failure.
    reserve(Charge::default())?;
    Ok(result)
}
