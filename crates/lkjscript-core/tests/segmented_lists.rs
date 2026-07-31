use std::num::{NonZeroU16, NonZeroU32};

use lkjscript_core::{
    decode_execution_outcome, encode_execution_outcome, ExecutionOutcome, OwnedValue,
    SegmentedListArena, SegmentedListArenaLimits, SegmentedListError, SegmentedListLimit, Value,
};

fn limits(segments: u32, entries: u32, capacity: u16) -> SegmentedListArenaLimits {
    SegmentedListArenaLimits::new(
        NonZeroU32::new(segments).unwrap_or(NonZeroU32::MIN),
        NonZeroU32::new(entries).unwrap_or(NonZeroU32::MIN),
        NonZeroU16::new(capacity).unwrap_or(NonZeroU16::MIN),
    )
}

#[test]
fn linear_prepend_uses_bounded_segments_and_preserves_order() -> Result<(), SegmentedListError> {
    let mut arena = SegmentedListArena::new(limits(3, 10, 4))?;
    let mut list = arena.empty();
    for value in 0..10_i64 {
        list = arena.prepend(value, list)?;
    }
    assert_eq!(
        arena.collect_cloned(list, 10)?,
        (0..10).rev().collect::<Vec<_>>()
    );
    assert_eq!(arena.metrics().live_entries, 10);
    assert_eq!(arena.metrics().live_segments, 3);
    assert_eq!(arena.metrics().segment_allocations, 3);
    Ok(())
}

#[test]
fn retained_and_branched_tails_share_entries_without_node_counts() -> Result<(), SegmentedListError>
{
    let mut arena = SegmentedListArena::new(limits(2, 8, 4))?;
    let empty = arena.empty();
    let one = arena.prepend(1_i64, empty)?;
    let two = arena.prepend(2, one)?;
    let left = arena.prepend(3, two)?;
    let right = arena.prepend(4, two)?;
    assert_eq!(arena.collect_cloned(two, 2)?, vec![2, 1]);
    assert_eq!(arena.collect_cloned(left, 3)?, vec![3, 2, 1]);
    assert_eq!(arena.collect_cloned(right, 3)?, vec![4, 2, 1]);
    assert_eq!(arena.rest(left)?, two);
    assert_eq!(arena.rest(right)?, two);
    assert_eq!(arena.metrics().live_segments, 1);
    Ok(())
}

#[test]
fn wrong_arena_empty_and_limits_fail_without_mutation() -> Result<(), SegmentedListError> {
    let mut first = SegmentedListArena::new(limits(1, 2, 2))?;
    let second = SegmentedListArena::<i64>::new(limits(1, 2, 2))?;
    assert_eq!(
        first.prepend(1, second.empty()),
        Err(SegmentedListError::WrongArena)
    );
    assert_eq!(first.metrics().live_entries, 0);
    let empty = first.empty();
    assert_eq!(
        first.first_cloned(empty),
        Err(SegmentedListError::EmptyList)
    );
    let one = first.prepend(1, empty)?;
    assert_eq!(
        second.key_from_word(one.to_word()),
        Err(SegmentedListError::WrongArena)
    );
    let two = first.prepend(2, one)?;
    let before = first.metrics();
    assert_eq!(
        first.prepend(3, two),
        Err(SegmentedListError::Limit(SegmentedListLimit::Entries))
    );
    assert_eq!(first.metrics(), before);
    Ok(())
}

#[test]
fn equality_counts_elements_not_segments_at_the_exact_bound() -> Result<(), SegmentedListError> {
    let mut arena = SegmentedListArena::new(limits(8, 16, 2))?;
    let mut left = arena.empty();
    let mut right = arena.empty();
    for value in 0..4_i64 {
        left = arena.prepend(value, left)?;
        right = arena.prepend(value, right)?;
    }
    assert!(arena.equal_by(left, right, 4, |a, b| a == b)?);
    assert_eq!(
        arena.equal_by(left, right, 3, |a, b| a == b),
        Err(SegmentedListError::Limit(
            SegmentedListLimit::TraversalSteps
        ))
    );
    let empty = arena.empty();
    let different = arena.prepend(9, empty)?;
    assert!(!arena.equal_by(left, different, 4, |a, b| a == b)?);
    Ok(())
}

#[test]
fn nested_segmented_lists_materialize_before_wire_snapshots() -> lkjscript_core::Result<()> {
    let mut lists = SegmentedListArena::new(limits(4, 8, 4))
        .map_err(|error| lkjscript_core::Error::msg(format!("list arena: {error:?}")))?;
    let empty = lists.empty();
    let inner = lists
        .prepend(Value::from_i64(7), empty)
        .map_err(|error| lkjscript_core::Error::msg(format!("inner list: {error:?}")))?;
    let outer = lists
        .prepend(Value::from_segmented_list(inner.to_word()), empty)
        .map_err(|error| lkjscript_core::Error::msg(format!("outer list: {error:?}")))?;
    let root = Value::from_segmented_list(outer.to_word());
    let owned = OwnedValue::from_segmented_list_snapshot(root, 8, |word| {
        let key = lists
            .key_from_word(word)
            .map_err(|error| lkjscript_core::Error::msg(format!("snapshot key: {error:?}")))?;
        lists
            .collect_cloned(key, 8)
            .map_err(|error| lkjscript_core::Error::msg(format!("snapshot list: {error:?}")))
    })?;
    assert_eq!(owned.list_len(), Some(1));
    assert_eq!(owned.snapshot_object_count(), 2);
    let outcome = ExecutionOutcome::Returned(owned);
    let encoded = encode_execution_outcome(&outcome, 64 * 1024)?;
    assert_eq!(decode_execution_outcome(&encoded, 64 * 1024)?, outcome);
    Ok(())
}
