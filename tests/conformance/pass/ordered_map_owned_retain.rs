use std::collections::{BTreeMap, BTreeSet};
pub fn probe() -> i64 {
    for pass in 0i32..3i32 {
        let mut map: BTreeMap<i32, i64> = BTreeMap::new();
        for index in 0i32..101i32 {
            let key = (index * 37i32) % 101i32;
            map.insert(key, key as i64 * 2i64);
        }
        let mut visited = 0i32;
        map.retain(|key: &i32, value: &mut i64| {
            if *key != visited { return false; }
            visited += 1i32;
            *value += 1i64;
            *key % 3i32 == pass
        });
        if visited != 101i32 { return 1; }
        match map.first_key_value() {
            Some((key, value)) => {
                if *key != pass || *value != pass as i64 * 2i64 + 1i64 { return 2; }
            }
            None => return 3,
        }
        if map.pop_first() != Some((pass, pass as i64 * 2i64 + 1i64)) { return 4; }
        let mut expected = pass + 3i32;
        for (key, value) in map {
            if key != expected || value != key as i64 * 2i64 + 1i64 { return 5; }
            expected += 3i32;
        }
        if expected < 101i32 { return 6; }
    }
    let mut set: BTreeSet<i32> = BTreeSet::new();
    for index in 0i32..101i32 { set.insert((index * 37i32) % 101i32); }
    if set.first() != Some(&0i32) { return 7; }
    if set.pop_first() != Some(0i32) { return 7; }
    let mut expected = 1i32;
    for key in set {
        if key != expected { return 8; }
        expected += 1i32;
    }
    if expected == 101i32 { 42 } else { 9 }
}
