//! The sealed cursor cannot borrow admission from an unrelated value or program.
use super::*;

#[test]
fn map_entry_cursor_rejects_foreign_origins_and_non_map_roots() {
    let (program, _) = fixture();
    let foreign = NormalizedProgram::prepare(program.artifact().clone()).unwrap();
    let mut checked = Value::empty_map(&program);
    assert!(checked.map_entries(&program).is_ok());
    assert!(checked.map_entries(&foreign).is_err());
    assert!(
        Value::scalar(&program, NormalizedValue::I64(0))
            .unwrap()
            .map_entries(&program)
            .is_err()
    );
    // Test-only corruption must not turn a non-free proof into free children.
    checked.class = Class::Direct;
    assert!(checked.map_entries(&program).is_err());
}

#[test]
fn map_entry_cursor_reserves_owned_spines_and_releases_interrupted_clones() {
    let (program, _) = fixture();
    let foreign = NormalizedProgram::prepare(program.artifact().clone()).unwrap();
    let mut work = ValueWork::default();
    let key: Arc<str> = Arc::from("key-immutable-日本語");
    let payload: Arc<str> = Arc::from("payload-immutable-日本語");
    for depth in [0, 1, 8, 32] {
        let mut child = Value::scalar(&program, NormalizedValue::Text(payload.clone())).unwrap();
        for _ in 0..depth {
            child = Value::option(&program, Some(child), &mut work).unwrap();
        }
        let parent = Value::empty_map(&program)
            .edit_map(
                &program,
                NormalizedMapKey::Text(key.clone()),
                Some(child),
                &mut work,
                &mut |_| Ok(()),
            )
            .unwrap();
        let expected = depth * std::mem::size_of::<NormalizedValue>() as u64;
        assert_eq!(Arc::strong_count(&key), 2);
        assert_eq!(Arc::strong_count(&payload), 2);
        for refused_call in 0..3 {
            let mut cursor = parent.map_entries(&program).unwrap();
            let mut requests = Vec::new();
            let rejected = cursor.next(&mut |charge| {
                let call = requests.len();
                requests.push(charge.bytes);
                if call == 1 {
                    assert_eq!(Arc::strong_count(&payload), 2, "reserve before clone");
                    assert_eq!(charge.bytes, expected);
                }
                if call == refused_call {
                    return Err(ExecutionError::resource("map_cursor_refusal", "test"));
                }
                Ok(())
            });
            assert_eq!(rejected.unwrap_err().code, "map_cursor_refusal");
            assert_eq!(requests.len(), refused_call + 1);
            assert_eq!(Arc::strong_count(&key), 2);
            assert_eq!(Arc::strong_count(&payload), 2);
        }
        let mut cursor = parent.map_entries(&program).unwrap();
        let mut requests = Vec::new();
        let (selected_key, selected) = cursor
            .next(&mut |charge| {
                requests.push(charge.bytes);
                Ok(())
            })
            .unwrap()
            .unwrap();
        assert_eq!(requests, [0, expected, 0]);
        assert_eq!(Arc::strong_count(&key), 3);
        assert_eq!(Arc::strong_count(&payload), 3);
        assert_eq!(selected_key.raw(), &NormalizedValue::Text(key.clone()));
        assert_eq!(selected.class(&program, &mut work).unwrap(), Class::Free);
        assert!(selected.class(&foreign, &mut work).is_err());
        assert!(cursor.next(&mut |_| Ok(())).unwrap().is_none());
        drop(parent);
        // The projected immutable values retain their own backing, not the map.
        assert_eq!(Arc::strong_count(&key), 2);
        assert_eq!(Arc::strong_count(&payload), 2);
        drop((selected_key, selected));
        assert_eq!(Arc::strong_count(&key), 1);
        assert_eq!(Arc::strong_count(&payload), 1);
    }
}
