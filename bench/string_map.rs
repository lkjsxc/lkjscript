//! Own qualified String keys, then perform compiler-like symbol-table operations.
mod common;
use std::collections::BTreeMap;

fn small_check() {
    let alpha = "module::alpha".to_string();
    let beta = "module::beta".to_string();
    let mut names: BTreeMap<String, u64> = BTreeMap::new();
    assert!(names.insert(beta.clone(), 22u64).is_none());
    assert!(names.insert(alpha.clone(), 20u64).is_none());
    assert!(*names.get(&alpha).unwrap() == 20u64);
    assert!(names.insert(alpha.clone(), 21u64) == Some(20u64));
    assert!(names.remove(&alpha) == Some(21u64));
    assert!(names.get(&alpha).is_none());
    assert!(names.insert(alpha, 20u64).is_none());
    let mut total = 0u64;
    let mut order = String::new();
    for (key, value) in names.iter() {
        total += *value;
        order.push_str(key);
    }
    assert!(names.len() == 2usize && total == 42u64);
    assert!(order == "module::alphamodule::beta");
}

pub fn main(mut world: World) {
    small_check();
    let arguments = world.args();
    let count = common::argument(&arguments, 0usize, 500000u64);
    let length = common::argument(&arguments, 1usize, 4096u64) as usize;
    let seed = common::argument(&arguments, 2usize, 7u64);
    assert!(length != 0usize);
    let mut keys: Vec<String> = Vec::new();
    let mut names: BTreeMap<String, u64> = BTreeMap::new();
    let mut index = 0usize;
    while index < length {
        let key = format!("module{}::item{}", index % 31usize, index);
        names.insert(key.clone(), (index as u64) ^ seed);
        keys.push(key);
        index += 1usize;
    }

    let mut checksum = 0u64;
    let mut state = seed;
    let mut operation = 0u64;
    while operation < count {
        state = state.wrapping_mul(1664525u64).wrapping_add(1013904223u64);
        let selected = (state % (length as u64)) as usize;
        let value = *names.get(&keys[selected]).unwrap();
        checksum = checksum.wrapping_add(value);
        if operation % 8u64 == 0u64 {
            let old = names.insert(keys[selected].clone(), value.wrapping_add(state)).unwrap();
            checksum = checksum.wrapping_add(old);
        } else if operation % 8u64 == 1u64 {
            let old = names.remove(&keys[selected]).unwrap();
            names.insert(keys[selected].clone(), old.wrapping_add(1u64));
            checksum = checksum.wrapping_add(old);
        }
        operation += 1u64;
    }
    assert!(names.len() == length);
    for (key, value) in names.iter() {
        checksum = checksum.wrapping_mul(33u64).wrapping_add(*value);
        checksum = checksum.wrapping_add(key.len() as u64);
    }
    let output = common::decimal(checksum);
    world.stdout().write(&output);
}
