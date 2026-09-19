//! Exact keyset bounds for one application-data namespace, space and part prefix.
//!
//! Bounds are private search sentinels, not admitted or persisted application keys.
//! In particular, appending a zero byte may exceed an admitted key's byte limit;
//! that sentinel only bounds an existing tree and can never be returned or stored.

use super::{DataEntry, DataKey, DataKeyPart, DataScanDirection, RecordKey};
use std::collections::{BTreeMap, btree_map};
use std::ops::Bound::{Excluded, Included};

pub(super) fn records<'a>(
    records: &'a BTreeMap<RecordKey, DataEntry>,
    namespace: &str,
    space: &str,
    prefix: &DataKey,
    direction: DataScanDirection,
    resume: Option<&DataKey>,
) -> btree_map::Range<'a, RecordKey, DataEntry> {
    let lower = RecordKey {
        namespace: namespace.to_owned(),
        space: space.to_owned(),
        key: prefix.clone(),
    };
    let upper = if prefix.parts().is_empty() {
        RecordKey {
            namespace: namespace.to_owned(),
            space: format!("{space}\0"),
            key: DataKey::empty_prefix(),
        }
    } else {
        RecordKey {
            namespace: namespace.to_owned(),
            space: space.to_owned(),
            key: prefix_end(prefix),
        }
    };
    let mut start = Included(lower.clone());
    let mut end = Excluded(upper.clone());
    if let Some(resume) = resume {
        let cursor = RecordKey {
            namespace: namespace.to_owned(),
            space: space.to_owned(),
            key: resume.clone(),
        };
        match direction {
            DataScanDirection::Forward => {
                if cursor >= upper {
                    return records.range(lower.clone()..lower);
                }
                if cursor >= lower {
                    start = Excluded(cursor);
                }
            }
            DataScanDirection::Reverse => {
                if cursor <= lower {
                    return records.range(lower.clone()..lower);
                }
                if cursor < upper {
                    end = Excluded(cursor);
                }
            }
        }
    }
    records.range((start, end))
}

/// The next part value is an exclusive bound on all extensions of this part
/// sequence. This is not a string-prefix operation: Text("a") and Text("ab")
/// are distinct key parts. A strict sequence prefix sorts before its extensions.
fn prefix_end(prefix: &DataKey) -> DataKey {
    let mut upper = prefix.clone();
    if let Some(last) = upper.0.last_mut() {
        match last {
            DataKeyPart::Bool(false) => *last = DataKeyPart::Bool(true),
            DataKeyPart::Bool(true) => *last = DataKeyPart::I64(i64::MIN),
            DataKeyPart::I64(i64::MAX) => *last = DataKeyPart::Text(String::new()),
            DataKeyPart::I64(value) => *value += 1,
            DataKeyPart::Text(value) => value.push('\0'),
            DataKeyPart::Bytes(value) => value.push(0),
        }
    }
    upper
}

// Count actual yielded records before the production iterator's filters. This
// source-bound oracle observes hidden rescans without changing production work
// accounting, limits, continuations, or release builds.
#[cfg(test)]
std::thread_local! {
    static VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn observe_visit() {
    VISITS.with(|visits| visits.set(visits.get().saturating_add(1)));
}

#[cfg(test)]
mod tests {
    use super::super::{
        DataExpectation, DataLimits, DataScanItem, DataScanPage, DataStore, DataTransaction,
        ScanSelector, encode_continuation,
    };
    use super::*;
    use std::cmp::Ordering;

    fn key(parts: Vec<DataKeyPart>) -> DataKey {
        DataKey::new(parts, &DataLimits::default()).expect("admitted fixture key")
    }

    fn part_order(left: &DataKeyPart, right: &DataKeyPart) -> Ordering {
        fn tag(part: &DataKeyPart) -> u8 {
            match part {
                DataKeyPart::Bool(_) => 0,
                DataKeyPart::I64(_) => 1,
                DataKeyPart::Text(_) => 2,
                DataKeyPart::Bytes(_) => 3,
            }
        }
        match (left, right) {
            (DataKeyPart::Bool(left), DataKeyPart::Bool(right)) => left.cmp(right),
            (DataKeyPart::I64(left), DataKeyPart::I64(right)) => left.cmp(right),
            (DataKeyPart::Text(left), DataKeyPart::Text(right)) => {
                left.as_bytes().cmp(right.as_bytes())
            }
            (DataKeyPart::Bytes(left), DataKeyPart::Bytes(right)) => left.cmp(right),
            _ => tag(left).cmp(&tag(right)),
        }
    }

    fn key_order(left: &DataKey, right: &DataKey) -> Ordering {
        for (left, right) in left.parts().iter().zip(right.parts()) {
            let order = part_order(left, right);
            if order != Ordering::Equal {
                return order;
            }
        }
        left.parts().len().cmp(&right.parts().len())
    }

    fn expected(
        transaction: &DataTransaction,
        space: &str,
        prefix: &[DataKeyPart],
        direction: DataScanDirection,
        resume: Option<&DataKey>,
    ) -> Vec<DataScanItem> {
        let mut expected = transaction
            .snapshot
            .records
            .iter()
            .filter(|(record, _)| {
                record.namespace == transaction.store.namespace
                    && record.space == space
                    && record.key.parts().len() >= prefix.len()
                    && record
                        .key
                        .parts()
                        .iter()
                        .zip(prefix)
                        .all(|(left, right)| part_order(left, right) == Ordering::Equal)
                    && resume.is_none_or(|resume| match direction {
                        DataScanDirection::Forward => {
                            key_order(&record.key, resume) == Ordering::Greater
                        }
                        DataScanDirection::Reverse => {
                            key_order(&record.key, resume) == Ordering::Less
                        }
                    })
            })
            .map(|(record, entry)| DataScanItem {
                key: record.key.clone(),
                value: entry.value.clone(),
                revision: entry.revision,
            })
            .collect::<Vec<_>>();
        expected.sort_by(|left, right| key_order(&left.key, &right.key));
        if direction == DataScanDirection::Reverse {
            expected.reverse();
        }
        expected
    }

    fn scan_page(
        transaction: &DataTransaction,
        prefix: &[DataKeyPart],
        direction: DataScanDirection,
        maximum_items: usize,
        maximum_work: usize,
        continuation: Option<&[u8]>,
    ) -> (DataScanPage, usize) {
        VISITS.with(|visits| visits.set(0));
        let page = transaction
            .scan(
                "facts",
                prefix,
                direction,
                maximum_items,
                1_048_576,
                maximum_work,
                continuation,
            )
            .expect("bounded page");
        let visits = VISITS.with(std::cell::Cell::get);
        assert!(
            visits <= page.items.len() + 1,
            "visited {visits} records to return {} items; unrelated or prior records were rescanned",
            page.items.len()
        );
        assert!(page.work <= maximum_work);
        assert!(page.items.len() <= maximum_items.min(maximum_work));
        (page, visits)
    }

    fn populated(keys: &[DataKey]) -> (tempfile::TempDir, DataTransaction) {
        let temporary = tempfile::TempDir::new().expect("temporary data fixture");
        let root = temporary.path().join("data");
        DataStore::initialize(&root).expect("initialize");
        let store = DataStore::open(&root, "app", DataLimits::default()).expect("open");
        let mut write = store.begin().expect("begin write");
        for (index, key) in keys.iter().enumerate() {
            assert!(
                write
                    .put(
                        "facts",
                        key,
                        format!("value-{index}").into_bytes(),
                        DataExpectation::Missing,
                    )
                    .expect("seed fact")
            );
        }
        write.commit().expect("commit facts");
        let other =
            DataStore::open(&root, "other", DataLimits::default()).expect("other namespace");
        let mut write = other.begin().expect("other write");
        write
            .put(
                "facts",
                &key(vec![DataKeyPart::I64(0)]),
                b"foreign namespace".to_vec(),
                DataExpectation::Missing,
            )
            .expect("other fact");
        write.commit().expect("other commit");
        let mut write = store.begin().expect("other space write");
        for space in ["fact", "facts-extra", "facts0"] {
            write
                .put(
                    space,
                    &key(vec![DataKeyPart::I64(0)]),
                    b"foreign space".to_vec(),
                    DataExpectation::Missing,
                )
                .expect("other space fact");
        }
        write.commit().expect("other space commit");
        (temporary, store.begin().expect("pinned reader"))
    }

    #[test]
    fn resumed_prefix_scan_examines_only_current_page() {
        let mut keys = Vec::new();
        for group in 0..64 {
            for index in 0..32 {
                keys.push(key(vec![DataKeyPart::I64(group), DataKeyPart::I64(index)]));
            }
        }
        let (_temporary, transaction) = populated(&keys);
        for direction in [DataScanDirection::Forward, DataScanDirection::Reverse] {
            for prefix in [vec![], vec![DataKeyPart::I64(48)]] {
                let oracle = expected(&transaction, "facts", &prefix, direction, None);
                let mut actual = Vec::new();
                let mut continuation = None;
                let mut pages = 0;
                let mut visits = 0;
                loop {
                    let (page, current_visits) = scan_page(
                        &transaction,
                        &prefix,
                        direction,
                        7,
                        5,
                        continuation.as_deref(),
                    );
                    pages += 1;
                    visits += current_visits;
                    actual.extend(page.items);
                    continuation = page.continuation;
                    if continuation.is_none() {
                        break;
                    }
                    assert!(pages <= oracle.len(), "continuation did not make progress");
                }
                assert_eq!(actual, oracle);
                assert!(visits <= actual.len() + pages);
            }
            for absent in [-1, 64, i64::MAX] {
                let (page, visits) = scan_page(
                    &transaction,
                    &[DataKeyPart::I64(absent)],
                    direction,
                    7,
                    5,
                    None,
                );
                assert!(page.items.is_empty());
                assert!(page.continuation.is_none());
                assert_eq!(visits, 0);
            }
        }
    }

    fn boundary_parts() -> Vec<DataKeyPart> {
        vec![
            DataKeyPart::Bool(false),
            DataKeyPart::Bool(true),
            DataKeyPart::I64(i64::MIN),
            DataKeyPart::I64(-1),
            DataKeyPart::I64(0),
            DataKeyPart::I64(1),
            DataKeyPart::I64(i64::MAX),
            DataKeyPart::Text(String::new()),
            DataKeyPart::Text("\0".to_owned()),
            DataKeyPart::Text("a".to_owned()),
            DataKeyPart::Text("a\0".to_owned()),
            DataKeyPart::Text("ab".to_owned()),
            DataKeyPart::Text("é".to_owned()),
            DataKeyPart::Text("\u{10ffff}".to_owned()),
            DataKeyPart::Bytes(vec![]),
            DataKeyPart::Bytes(vec![0]),
            DataKeyPart::Bytes(vec![0, 0]),
            DataKeyPart::Bytes(vec![255]),
            DataKeyPart::Bytes(vec![255, 0]),
        ]
    }

    #[test]
    fn all_part_boundaries_and_prefixes_match_independent_flat_oracle() {
        let mut keys = Vec::new();
        for part in boundary_parts() {
            keys.push(key(vec![part.clone()]));
            for suffix in [
                DataKeyPart::Bool(false),
                DataKeyPart::I64(-1),
                DataKeyPart::Text("leaf".to_owned()),
                DataKeyPart::Bytes(vec![255]),
            ] {
                keys.push(key(vec![part.clone(), suffix]));
            }
        }
        let (_temporary, transaction) = populated(&keys);
        let mut prefixes = vec![vec![]];
        prefixes.extend(keys.iter().map(|key| key.parts().to_vec()));
        prefixes.extend([
            vec![DataKeyPart::I64(42)],
            vec![DataKeyPart::Text("missing".to_owned())],
            vec![DataKeyPart::Bytes(vec![255, 255])],
        ]);
        for prefix in prefixes {
            for direction in [DataScanDirection::Forward, DataScanDirection::Reverse] {
                let oracle = expected(&transaction, "facts", &prefix, direction, None);
                for (maximum_items, maximum_work) in [(1, 1), (3, 4), (9, 2)] {
                    let mut actual = Vec::new();
                    let mut continuation = None;
                    let mut pages = 0;
                    loop {
                        let (page, _) = scan_page(
                            &transaction,
                            &prefix,
                            direction,
                            maximum_items,
                            maximum_work,
                            continuation.as_deref(),
                        );
                        pages += 1;
                        actual.extend(page.items);
                        continuation = page.continuation;
                        if continuation.is_none() {
                            break;
                        }
                        assert!(pages <= oracle.len(), "continuation did not make progress");
                    }
                    assert_eq!(actual, oracle, "prefix {prefix:?}, direction {direction:?}");
                }
            }
        }
    }

    #[test]
    fn rehashed_out_of_range_resume_keys_cannot_invert_tree_bounds() {
        let keys = boundary_parts()
            .into_iter()
            .map(|part| key(vec![part]))
            .collect::<Vec<_>>();
        let (_temporary, transaction) = populated(&keys);
        for prefix in [
            vec![],
            vec![DataKeyPart::Bool(false)],
            vec![DataKeyPart::I64(0)],
            vec![DataKeyPart::I64(i64::MAX)],
            vec![DataKeyPart::Text("a".to_owned())],
            vec![DataKeyPart::Bytes(vec![255])],
        ] {
            for direction in [DataScanDirection::Forward, DataScanDirection::Reverse] {
                for resume in &keys {
                    let selector = ScanSelector {
                        store_id: transaction.store.store_id,
                        revision: transaction.base,
                        namespace: &transaction.store.namespace,
                        space: "facts",
                        prefix: &DataKey(prefix.clone()),
                        direction,
                        maximum_items: 100,
                        maximum_bytes: 1_048_576,
                        maximum_work: 100,
                        resume: None,
                    };
                    let token = encode_continuation(&selector, resume).expect("rehashed token");
                    let oracle = expected(&transaction, "facts", &prefix, direction, Some(resume));
                    let (page, _) =
                        scan_page(&transaction, &prefix, direction, 100, 100, Some(&token));
                    assert_eq!(page.items, oracle);
                    assert!(page.continuation.is_none());
                }
            }
        }
    }

    #[test]
    fn private_search_sentinels_do_not_change_admitted_key_limits() {
        let mut parts = vec![DataKeyPart::Bool(true); 15];
        parts.push(DataKeyPart::Text("edge".to_owned()));
        let deepest = key(parts);
        // Four count bytes, one type tag, eight length bytes, then the payload.
        let longest = key(vec![DataKeyPart::Bytes(vec![255; 4083])]);
        assert_eq!(
            super::super::encode_key(&longest)
                .expect("encode exact boundary")
                .len(),
            4096
        );
        assert!(
            DataKey::new(
                vec![DataKeyPart::Bytes(vec![255; 4084])],
                &DataLimits::default()
            )
            .is_err()
        );
        let (_temporary, transaction) = populated(&[deepest.clone(), longest.clone()]);
        for selected in [deepest, longest] {
            for direction in [DataScanDirection::Forward, DataScanDirection::Reverse] {
                let (page, visits) =
                    scan_page(&transaction, selected.parts(), direction, 1, 1, None);
                assert_eq!(page.items.len(), 1);
                assert_eq!(page.items[0].key, selected);
                assert_eq!(visits, 1);
                assert!(page.continuation.is_none());
            }
        }
    }

    #[test]
    fn byte_caps_and_lookahead_accounting_are_preserved_after_seeking() {
        let keys = (0..3)
            .map(|value| key(vec![DataKeyPart::I64(value)]))
            .collect::<Vec<_>>();
        let (_temporary, transaction) = populated(&keys);
        // One part count (4), integer tag/payload (1+8), "value-N" (7), revision (32).
        const ITEM_BYTES: usize = 52;
        for direction in [DataScanDirection::Forward, DataScanDirection::Reverse] {
            VISITS.with(|visits| visits.set(0));
            let error = transaction
                .scan("facts", &[], direction, 10, ITEM_BYTES - 1, 10, None)
                .expect_err("the first selected row cannot be silently skipped");
            assert_eq!(error.code, "data_scan_item_bytes");
            assert_eq!(VISITS.with(std::cell::Cell::get), 1);
            let mut cursor = None;
            let mut actual = Vec::new();
            for page_index in 0..3 {
                VISITS.with(|visits| visits.set(0));
                let page = transaction
                    .scan(
                        "facts",
                        &[],
                        direction,
                        10,
                        ITEM_BYTES,
                        10,
                        cursor.as_deref(),
                    )
                    .expect("one exact-size record");
                assert_eq!(page.bytes, ITEM_BYTES);
                assert_eq!(page.items.len(), 1);
                let examined = if page_index < 2 { 2 } else { 1 };
                assert_eq!(page.work, examined, "byte-bound lookahead is charged");
                assert_eq!(VISITS.with(std::cell::Cell::get), examined);
                actual.extend(page.items);
                cursor = page.continuation;
                assert_eq!(cursor.is_some(), page_index < 2);
            }
            assert_eq!(
                actual,
                expected(&transaction, "facts", &[], direction, None)
            );
            let (page, visits) = scan_page(&transaction, &[], direction, 1, 1, None);
            assert_eq!(page.bytes, ITEM_BYTES);
            assert_eq!(page.work, 1, "work-limit lookahead is not charged");
            assert_eq!(visits, 2);
            let page = transaction
                .scan("facts", &[], direction, 10, 2 * ITEM_BYTES, 10, None)
                .expect("two exact-size records");
            assert_eq!(page.items.len(), 2);
            assert_eq!(page.bytes, 2 * ITEM_BYTES);
            assert_eq!(page.work, 3);
            assert!(page.continuation.is_some());
        }
    }

    #[test]
    fn deleted_cursors_staged_mutations_and_pinned_snapshots_keep_keyset_semantics() {
        let keys = [0, 2, 4, 6].map(|value| key(vec![DataKeyPart::I64(value)]));
        let (temporary, mut transaction) = populated(&keys);
        let store = DataStore::open(&temporary.path().join("data"), "app", DataLimits::default())
            .expect("second store handle");
        let pinned = store.begin().expect("pin committed predecessor");
        let (first, _) = scan_page(&transaction, &[], DataScanDirection::Forward, 1, 1, None);
        let cursor = first.continuation.expect("first key continuation");
        assert_eq!(first.items[0].key, keys[0]);
        for selected in [&keys[0], &keys[2]] {
            let entry = transaction
                .get("facts", selected)
                .expect("get")
                .expect("existing");
            assert!(
                transaction
                    .delete("facts", selected, DataExpectation::Exact(entry.revision))
                    .expect("stage deletion, including cursor key")
            );
        }
        for value in [-1, 1] {
            assert!(
                transaction
                    .put(
                        "facts",
                        &key(vec![DataKeyPart::I64(value)]),
                        b"inserted".to_vec(),
                        DataExpectation::Missing
                    )
                    .expect("stage before and after cursor")
            );
        }
        let entry = transaction
            .get("facts", &keys[1])
            .expect("get")
            .expect("existing");
        assert!(
            transaction
                .put(
                    "facts",
                    &keys[1],
                    b"updated".to_vec(),
                    DataExpectation::Exact(entry.revision)
                )
                .expect("stage update after cursor")
        );
        let mut actual = Vec::new();
        let mut next = Some(cursor.clone());
        while let Some(token) = next {
            let (page, _) = scan_page(
                &transaction,
                &[],
                DataScanDirection::Forward,
                1,
                1,
                Some(&token),
            );
            actual.extend(page.items);
            next = page.continuation;
            assert!(actual.len() <= 3, "no duplicate or backwards progress");
        }
        assert_eq!(
            actual
                .iter()
                .map(|item| item.key.clone())
                .collect::<Vec<_>>(),
            [1, 2, 6].map(|value| key(vec![DataKeyPart::I64(value)]))
        );
        assert_eq!(actual[0].value, b"inserted");
        assert_eq!(actual[1].value, b"updated");
        transaction.commit().expect("publish staged snapshot");
        let (old_page, _) = scan_page(
            &pinned,
            &[],
            DataScanDirection::Forward,
            1,
            1,
            Some(&cursor),
        );
        assert_eq!(old_page.items[0].key, keys[1]);
        assert_eq!(old_page.items[0].value, b"value-1");
        assert_ne!(old_page.items[0].revision, actual[1].revision);
        let fresh = store.begin().expect("new committed snapshot");
        assert_eq!(
            fresh
                .scan(
                    "facts",
                    &[],
                    DataScanDirection::Forward,
                    1,
                    1_048_576,
                    1,
                    Some(&cursor)
                )
                .expect_err("old cursor cannot authorize a new snapshot")
                .code,
            "data_continuation_selector"
        );
        let (page, _) = scan_page(&fresh, &[], DataScanDirection::Forward, 10, 10, None);
        assert_eq!(
            page.items
                .iter()
                .map(|item| item.key.clone())
                .collect::<Vec<_>>(),
            [-1, 1, 2, 6].map(|value| key(vec![DataKeyPart::I64(value)]))
        );
        assert!(page.continuation.is_none());
    }
}
