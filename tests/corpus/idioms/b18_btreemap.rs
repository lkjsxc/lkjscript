use std::collections::BTreeMap;
pub fn probe() -> i64 { let mut m: BTreeMap<i64, i64> = BTreeMap::new(); m.insert(1, 40); m.insert(2, 2); let mut s: i64 = 0; if let Some(v) = m.get(&1) { s += *v; } if m.contains_key(&2) { s += 2; } s }
