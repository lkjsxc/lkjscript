use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Default)]
struct State {
    map: BTreeMap<i32, String>,
    set: BTreeSet<i32>,
}
pub fn probe() -> i64 {
    let mut state = State::default();
    state.map.insert(3, "three".to_string());
    state.map.insert(1, "one".to_string());
    state.map.insert(2, "two".to_string());
    state.set.insert(3);
    state.set.insert(1);
    let copy = state.clone();
    state.map.remove(&2);
    state.map.insert(2, "two".to_string());
    let mut other: BTreeMap<i32, String> = BTreeMap::new();
    other.insert(2, "two".to_string());
    other.insert(1, "one".to_string());
    other.insert(3, "three".to_string());
    if state.map == copy.map && state.map == other && state.set == copy.set { 42 } else { 0 }
}
