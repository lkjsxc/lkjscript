//! Independent complete-result checks and a lookup-free production traversal law.
use super::*;

fn assert_integer_entries(value: &NormalizedValue, count: i64) {
    let NormalizedValue::List(entries) = value else {
        panic!("entry projection must return a list")
    };
    assert_eq!(entries.len(), count as usize);
    for (index, value) in entries.iter().enumerate() {
        let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
            panic!("each projected entry must be a structural record")
        };
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0.as_str(), "key");
        assert_eq!(fields[0].1, NormalizedValue::I64(index as i64));
        assert_eq!(fields[1].0.as_str(), "value");
        assert_eq!(fields[1].1, NormalizedValue::I64(13 - 7 * index as i64));
    }
}

#[test]
fn map_entries_native_tree_scan_is_linear_after_admission() {
    let (program, snapshot) = fixture();
    let schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let reader = boundary_reader(&snapshot, &schema);
    let integer = scalar_type(&program, TypeForm::I64);
    let mut production = Vec::new();
    for count in [0, 1, 31, 32, 33, 1024, 4096] {
        // Unequal keys and payloads detect key-only or swapped-column results.
        // The BTreeMap helper canonicalizes insertion order; separate update
        // tests construct distinct persistent-tree shapes.
        let original = map((0..count).rev().map(|key| {
            (
                NormalizedMapKey::I64(key),
                NormalizedValue::I64(13 - 7 * key),
            )
        }));
        for reference in [false, true] {
            let control = ExecutionControl::uncancelled();
            let length = invoke(
                &program,
                &reader,
                reference,
                "core.map.length",
                &[integer, integer],
                vec![original.clone()],
                NormalizedRunPolicy::foreground(),
                &control,
                None,
            );
            assert_eq!(length.value.unwrap(), NormalizedValue::I64(count));
            let entries = invoke(
                &program,
                &reader,
                reference,
                "core.map.entries",
                &[integer, integer],
                vec![original.clone()],
                NormalizedRunPolicy::foreground(),
                &control,
                None,
            );
            assert_integer_entries(entries.value.as_ref().unwrap(), count);
            assert_eq!(
                entries.work.input_admission_nodes, length.work.input_admission_nodes,
                "complete raw admission is identical to a header-only length query"
            );
            let scan = entries
                .work
                .maps
                .node_visits
                .checked_sub(length.work.maps.node_visits)
                .unwrap();
            eprintln!(
                "map_entry_scan count={count} reference={reference} admission_visits={} total_visits={} scan_visits={scan}",
                length.work.maps.node_visits, entries.work.maps.node_visits
            );
            if !reference {
                production.push((count, scan));
                assert_eq!(
                    entries.work.raw_result_admission_nodes, 0,
                    "checked projection must not fall back to recursive raw admission"
                );
            }
        }
        let NormalizedValue::Map(retained) = &original else {
            unreachable!()
        };
        assert_eq!(retained.len(), count as usize);
        for (key, value) in retained.iter() {
            let NormalizedMapKey::I64(key) = key else {
                unreachable!()
            };
            assert_eq!(*value, NormalizedValue::I64(13 - 7 * key));
        }
    }
    for (count, scan) in production {
        assert_eq!(
            scan, count as u64,
            "entry enumeration must visit each retained node once, not search from the root for every key (count={count})"
        );
    }
}
