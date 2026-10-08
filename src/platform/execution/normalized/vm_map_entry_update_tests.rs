//! Actual path-copy updates exercise shapes that bulk sorted construction cannot.
use super::*;

#[test]
fn map_entries_path_updates_preserve_every_retained_version_in_distinct_shapes() {
    let (program, snapshot) = fixture();
    let integer = scalar_type(&program, TypeForm::I64);
    for count in [0_i64, 1, 31, 32, 33, 255, 1024] {
        for shape in 0..4 {
            let mut order: Vec<_> = (0..count).collect();
            match shape {
                1 => order.reverse(),
                2 => order.sort_by_key(|key| (key % 2, *key)),
                3 => order.sort_by_key(|key| (key * 37) % count.max(1)),
                _ => {}
            }
            let mut tree = normalized::map::Map::default();
            let mut oracle = BTreeMap::new();
            for key in order {
                let value = NormalizedValue::I64(13 - 7 * key);
                tree = tree
                    .insert(
                        NormalizedMapKey::I64(key),
                        value.clone(),
                        1_000_000,
                        &mut |_| Ok(()),
                    )
                    .unwrap();
                oracle.insert(key, value);
            }
            let original = tree.clone();
            let original_oracle = oracle.clone();
            for key in (0..count).step_by(3) {
                tree = tree
                    .remove(&NormalizedMapKey::I64(key), &mut |_| Ok(()))
                    .unwrap();
                oracle.remove(&key);
            }
            for key in (0..count).step_by(5).chain([i64::MIN, i64::MAX]) {
                let value = NormalizedValue::I64(key.wrapping_mul(11));
                tree = tree
                    .insert(
                        NormalizedMapKey::I64(key),
                        value.clone(),
                        1_000_000,
                        &mut |_| Ok(()),
                    )
                    .unwrap();
                oracle.insert(key, value);
            }
            for (tree, oracle) in [(original, original_oracle), (tree, oracle)] {
                let raw = NormalizedValue::Map(tree);
                for reference in [false, true] {
                    let observed = invoke(
                        &program,
                        &snapshot,
                        reference,
                        "core.map.entries",
                        &[integer, integer],
                        vec![raw.clone()],
                        NormalizedRunPolicy::foreground(),
                        &ExecutionControl::uncancelled(),
                        None,
                    );
                    let NormalizedValue::List(entries) = observed.value.unwrap() else {
                        panic!("list")
                    };
                    assert_eq!(entries.len(), oracle.len());
                    for (actual, (key, value)) in entries.iter().zip(&oracle) {
                        let NormalizedValue::Record(NormalizedRecord::Structural { fields }) =
                            actual
                        else {
                            panic!("entry")
                        };
                        assert_eq!(fields.len(), 2);
                        assert_eq!(
                            fields[0],
                            (Name::new("key").unwrap(), NormalizedValue::I64(*key))
                        );
                        assert_eq!(&fields[1], &(Name::new("value").unwrap(), value.clone()));
                    }
                    if !reference {
                        // Raw map admission visits n nodes; projection visits n more.
                        assert_eq!(observed.work.maps.node_visits, 2 * oracle.len() as u64);
                        assert_eq!(observed.work.raw_result_admission_nodes, 0);
                    }
                }
            }
        }
    }
}
