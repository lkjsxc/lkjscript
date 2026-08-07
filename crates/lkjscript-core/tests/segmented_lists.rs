#![allow(clippy::expect_used)]

use lkjscript_core::{OwnedValue, SegmentedListArena, SegmentedListError, Value};

#[test]
fn linear_prepend_crosses_former_entry_limit_and_preserves_order() -> Result<(), SegmentedListError>
{
    const COUNT: i64 = 65_537;
    let mut arena = SegmentedListArena::new()?;
    let mut list = arena.empty();
    for value in 0..COUNT {
        list = arena.prepend(value, list)?;
    }
    let values = arena.collect_cloned(list)?;
    assert_eq!(
        values.len(),
        usize::try_from(COUNT).expect("test count fits usize")
    );
    assert_eq!(values.first(), Some(&(COUNT - 1)));
    assert_eq!(values.last(), Some(&0));
    assert_eq!(
        arena.metrics().live_entries,
        u64::try_from(COUNT).map_err(|_| SegmentedListError::Limit(
            lkjscript_core::SegmentedListLimit::Representation
        ))?
    );
    Ok(())
}

#[test]
fn retained_and_branched_tails_share_entries() -> Result<(), SegmentedListError> {
    let mut arena = SegmentedListArena::new()?;
    let empty = arena.empty();
    let one = arena.prepend(1_i64, empty)?;
    let two = arena.prepend(2, one)?;
    let left = arena.prepend(3, two)?;
    let right = arena.prepend(4, two)?;
    assert_eq!(arena.collect_cloned(two)?, vec![2, 1]);
    assert_eq!(arena.collect_cloned(left)?, vec![3, 2, 1]);
    assert_eq!(arena.collect_cloned(right)?, vec![4, 2, 1]);
    assert_eq!(arena.rest(left)?, two);
    assert_eq!(arena.rest(right)?, two);
    Ok(())
}

#[test]
fn wrong_arena_and_empty_fail_without_mutation() -> Result<(), SegmentedListError> {
    let mut first = SegmentedListArena::new()?;
    let second = SegmentedListArena::<i64>::new()?;
    assert_eq!(
        first.prepend(1, second.empty()),
        Err(SegmentedListError::WrongArena)
    );
    assert_eq!(first.metrics().live_entries, 0);
    assert_eq!(
        first.first_cloned(first.empty()),
        Err(SegmentedListError::EmptyList)
    );
    Ok(())
}

#[test]
fn nested_segmented_lists_materialize_into_owned_snapshots() -> lkjscript_core::Result<()> {
    let mut lists = SegmentedListArena::new()
        .map_err(|error| lkjscript_core::Error::msg(format!("list arena: {error:?}")))?;
    let empty = lists.empty();
    let inner = lists
        .prepend(Value::from_i64(7), empty)
        .map_err(|error| lkjscript_core::Error::msg(format!("inner list: {error:?}")))?;
    let outer = lists
        .prepend(Value::from_segmented_list(inner.to_word()), empty)
        .map_err(|error| lkjscript_core::Error::msg(format!("outer list: {error:?}")))?;
    let root = Value::from_segmented_list(outer.to_word());
    let owned = OwnedValue::from_segmented_list_snapshot(root, |word| {
        let key = lists
            .key_from_word(word)
            .map_err(|error| lkjscript_core::Error::msg(format!("snapshot key: {error:?}")))?;
        lists
            .collect_cloned(key)
            .map_err(|error| lkjscript_core::Error::msg(format!("snapshot list: {error:?}")))
    })?;
    assert_eq!(owned.list_len(), Some(1));
    assert_eq!(owned.snapshot_object_count(), 2);
    Ok(())
}
