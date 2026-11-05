//! Focused storage controls, not source-semantic or allocator-traffic claims.
//! The owner-factory refusal is deliberately injected after payload assembly.

use super::*;
use crate::platform::execution::normalized::structural_equality::assertions::equal;
use crate::platform::execution::normalized::value::{FunctionIndex, ValueOrigin};
use std::borrow::Cow;
use std::sync::Arc as StdArc;
use std::sync::Barrier;
use std::sync::atomic::Ordering;
use std::thread;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn bare_function() -> NormalizedValue {
    NormalizedValue::Function {
        function: FunctionIndex(0, ValueOrigin::default()),
        type_arguments: Default::default(),
        effect_arguments: Default::default(),
        requirement_arguments: Default::default(),
        read_scope_arguments: Default::default(),
        bound_arguments: None,
    }
}

fn nested(
    mut value: NormalizedValue,
    depth: usize,
    named: bool,
    probe: &StdArc<DropProbe>,
) -> TestResult<NormalizedValue> {
    let template = bare_function();
    for _ in 0..depth {
        if named {
            let mut fields = FieldsBuilder::with_capacity(1)?;
            fields.node.probe = Some(StdArc::clone(probe));
            let name = Name::new("f").map_err(|error| error.message)?;
            fields.try_push((name, value))?;
            value = NormalizedValue::Record(NormalizedRecord::Structural {
                fields: fields.finish()?,
            });
        }
        let mut prefix = ValuesBuilder::with_capacity(1)?;
        prefix.node.probe = Some(StdArc::clone(probe));
        prefix.try_push(value)?;
        let captures = Captures::from_values(prefix.finish()?);
        let mut next = template.clone();
        let NormalizedValue::Function {
            bound_arguments, ..
        } = &mut next
        else {
            return Err("test template is not a function".into());
        };
        *bound_arguments = Some(captures);
        value = next;
    }
    Ok(value)
}

fn drop_on_small_stack(value: impl Send + 'static) -> TestResult {
    let worker = thread::Builder::new()
        .name("opaque-child-drop".into())
        .stack_size(64 * 1024)
        .spawn(move || drop(value))?;
    worker
        .join()
        .map_err(|_| "opaque child drop worker panicked")?;
    Ok(())
}

#[test]
fn opaque_children_deep_function_and_record_drop() -> TestResult {
    for named in [false, true] {
        let probe = StdArc::new(DropProbe::default());
        let depth = 16_384;
        let value = nested(bare_function(), depth, named, &probe)?;
        drop_on_small_stack(value)?;
        assert_eq!(
            probe.finished.load(Ordering::Relaxed),
            depth * if named { 2 } else { 1 }
        );
        assert!(probe.maximum_depth.load(Ordering::Relaxed) <= 2);
    }
    Ok(())
}

#[test]
fn opaque_children_wide_shared_dag_keeps_survivor() -> TestResult {
    let probe = StdArc::new(DropProbe::default());
    let depth = 512;
    let width = 512;
    let shared = nested(bare_function(), depth, false, &probe)?;
    let survivor = shared.clone();
    let mut root = ValuesBuilder::with_capacity(width)?;
    root.node.probe = Some(StdArc::clone(&probe));
    for _ in 0..width {
        root.try_push(nested(shared.clone(), 1, false, &probe)?)?;
    }
    drop(shared);
    let root = root.finish()?;
    drop_on_small_stack(root)?;
    assert_eq!(probe.finished.load(Ordering::Relaxed), width + 1);
    let NormalizedValue::Function {
        bound_arguments: Some(prefix),
        ..
    } = &survivor
    else {
        return Err("surviving owner lost its callable payload".into());
    };
    assert_eq!(prefix.len(), 1);
    drop_on_small_stack(survivor)?;
    assert_eq!(probe.finished.load(Ordering::Relaxed), depth + width + 1);
    assert!(probe.maximum_depth.load(Ordering::Relaxed) <= 2);
    Ok(())
}

#[test]
fn opaque_children_concurrent_final_release_is_exact() -> TestResult {
    let probe = StdArc::new(DropProbe::default());
    let depth = 4_096;
    let first = nested(bare_function(), depth, true, &probe)?;
    let second = first.clone();
    let start = StdArc::new(Barrier::new(2));
    let left_start = StdArc::clone(&start);
    let left = thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || {
            left_start.wait();
            drop(first);
        })?;
    start.wait();
    drop(second);
    left.join()
        .map_err(|_| "left final-release worker panicked")?;
    assert_eq!(probe.finished.load(Ordering::Relaxed), depth * 2);
    assert!(probe.maximum_depth.load(Ordering::Relaxed) <= 2);
    Ok(())
}

#[test]
fn opaque_children_refused_payload_extent_keeps_existing_owner() -> TestResult {
    let probe = StdArc::new(DropProbe::default());
    let retained = nested(bare_function(), 64, false, &probe)?;
    let mut partial = ValuesBuilder::with_capacity(1)?;
    partial.node.probe = Some(StdArc::clone(&probe));
    partial.try_push(retained.clone())?;
    assert_eq!(partial.try_reserve(usize::MAX), Err(StorageFailure::Extent));
    assert_eq!(partial.node.items.len(), 1);
    drop(partial);
    assert_eq!(probe.finished.load(Ordering::Relaxed), 1);
    let NormalizedValue::Function {
        bound_arguments: Some(prefix),
        ..
    } = &retained
    else {
        return Err("extent refusal damaged the retained callable".into());
    };
    assert_eq!(prefix.len(), 1);
    drop(retained);
    assert_eq!(probe.finished.load(Ordering::Relaxed), 65);
    Ok(())
}

#[test]
fn opaque_children_refused_owner_factory_drains_completed_payload() -> TestResult {
    let probe = StdArc::new(DropProbe::default());
    let retained = nested(bare_function(), 64, false, &probe)?;
    let mut partial = ValuesBuilder::with_capacity(1)?;
    partial.node.probe = Some(StdArc::clone(&probe));
    partial.try_push(retained.clone())?;
    let refusal = refuse_owner_factory_after(0);
    let refused = partial.finish();
    assert!(matches!(refused, Err(StorageFailure::Allocation)));
    drop(refusal);
    // Restoration is observable through the same production factory path.
    drop(Values::try_new(Vec::new())?);
    assert_eq!(probe.finished.load(Ordering::Relaxed), 1);
    let NormalizedValue::Function {
        bound_arguments: Some(prefix),
        ..
    } = &retained
    else {
        return Err("owner allocation refusal damaged the retained callable".into());
    };
    assert_eq!(prefix.len(), 1);
    drop(retained);
    assert_eq!(probe.finished.load(Ordering::Relaxed), 65);

    // The injected refusal verifies teardown ordering, not physical OOM.
    // Its final error carrier itself borrows static text.
    let error = ExecutionError::from(StorageFailure::Allocation);
    assert!(matches!(error.code, Cow::Borrowed(_)));
    assert!(matches!(error.message, Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn opaque_children_structural_comparison_and_bounded_debug() -> TestResult {
    let probe = StdArc::new(DropProbe::default());
    let value = nested(bare_function(), 4_096, true, &probe)?;
    let cloned = value.clone();
    assert_eq!(equal(&value, &cloned), Ok(true));
    let text = format!("{value:?}");
    assert!(text.contains("Captures { len: 1 }"));
    assert!(text.len() < 2_048);

    let left = Captures::try_new(vec![NormalizedValue::I64(7)])?;
    let alias = left.clone();
    let right = Captures::try_new(vec![NormalizedValue::I64(7)])?;
    assert_eq!(equal(&left, &alias), Ok(true));
    assert!(left.ptr_eq(&alias));
    assert!(!left.ptr_eq(&right));
    assert_eq!(equal(&left, &right), Ok(true));
    let first_record = Values::try_new(vec![NormalizedValue::I64(7)])?;
    let second_record = Values::try_new(vec![NormalizedValue::I64(7)])?;
    assert_eq!(equal(&first_record, &second_record), Ok(true));
    drop_on_small_stack(value)?;
    assert_eq!(probe.finished.load(Ordering::Relaxed), 0);
    drop_on_small_stack(cloned)?;
    assert_eq!(probe.finished.load(Ordering::Relaxed), 8_192);
    assert!(probe.maximum_depth.load(Ordering::Relaxed) <= 2);
    Ok(())
}

#[test]
fn opaque_children_mixed_list_map_record_drop_keeps_shared_survivor() -> TestResult {
    use crate::platform::execution::normalized::value::NormalizedMapKey;
    use std::collections::BTreeMap;

    let probe = StdArc::new(DropProbe::default());
    let payload: StdArc<str> = StdArc::from("retained-leaf");
    let mut value = NormalizedValue::Text(StdArc::clone(&payload));
    let depth = 4_096;
    for _ in 0..depth {
        value = nested(value, 1, true, &probe)?;
        value = NormalizedValue::list(vec![value]).map_err(|error| format!("{error:?}"))?;
        value = NormalizedValue::map(BTreeMap::from([
            (NormalizedMapKey::I64(0), value),
            (NormalizedMapKey::I64(1), NormalizedValue::I64(9)),
        ]))
        .map_err(|error| format!("{error:?}"))?;
    }
    let survivor = value.clone();
    drop_on_small_stack(value)?;
    assert_eq!(probe.finished.load(Ordering::Relaxed), 0);
    assert_eq!(StdArc::strong_count(&payload), 2);
    drop_on_small_stack(survivor)?;
    assert_eq!(probe.finished.load(Ordering::Relaxed), depth * 2);
    assert_eq!(StdArc::strong_count(&payload), 1);
    assert!(probe.maximum_depth.load(Ordering::Relaxed) <= 2);
    Ok(())
}

#[test]
fn opaque_named_children_sort_preserves_equal_name_order() -> TestResult {
    let mut builder = FieldsBuilder::with_capacity(100)?;
    for index in 0..100_i64 {
        let name = Name::new(format!("k{}", index % 3)).map_err(|error| error.message)?;
        builder.try_push((name, NormalizedValue::I64(index)))?;
    }
    builder.sort_by_name()?;
    let fields = builder.finish()?;
    for pair in fields.windows(2) {
        assert!(pair[0].0 <= pair[1].0);
        if pair[0].0 == pair[1].0 {
            let (NormalizedValue::I64(left), NormalizedValue::I64(right)) =
                (&pair[0].1, &pair[1].1)
            else {
                return Err("sort changed a field payload".into());
            };
            assert!(left < right);
        }
    }
    Ok(())
}
