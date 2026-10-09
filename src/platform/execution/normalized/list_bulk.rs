//! Assemble only final immutable nodes; no published prefix needs path copying.
use super::super::value::{NormalizedValue, RawArguments};
use super::{
    Charge, FANOUT, List, MAXIMUM_LENGTH, Node, branch, element, failure, invalid, leaf, node,
};
use crate::platform::execution::ExecutionError;
use std::sync::Arc;

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
    // Recursion is bounded by the existing checked storage height, not input
    // nesting. Partial branches keep the requested height for append/indexing.
    if leaves == 0 || leaves > List::capacity(height)? {
        return Err(invalid());
    }
    if height == 0 {
        return build_leaf(pending, FANOUT, reserve);
    }
    let span = List::capacity(height - 1)?;
    let mut children = branch(None, reserve)?;
    let mut remaining = leaves;
    for child in &mut children {
        if remaining == 0 {
            break;
        }
        let count = remaining.min(span);
        *child = Some(build_prefix(pending, height - 1, count, reserve)?);
        remaining -= count;
    }
    if remaining != 0 {
        return Err(invalid());
    }
    Ok(node(Node::Branch(children)))
}

pub(super) fn from_items(
    items: Vec<NormalizedValue>,
    maximum_length: u64,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<List, ExecutionError> {
    let mut pending = RawArguments::new(items);
    let length = pending.len();
    if length > MAXIMUM_LENGTH || length as u64 > maximum_length {
        return Err(failure());
    }
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
