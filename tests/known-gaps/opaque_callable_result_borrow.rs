// Definition-site origin gap: B may carry the borrow of value, so value cannot
// be assigned after the call while result remains live. Rust rejects this
// unused generic definition with E0506. lkjscript currently accepts the unused
// definition because its rigid B has no symbolic reference inventory.
// Keep this outside accepted conformance until the origin-family contract lands.
fn invalid<'a, B, F: FnMut(&'a mut i64) -> B>(mut function: F, value: &'a mut i64) -> B {
    let result = function(value);
    *value = 0;
    result
}

pub fn probe() -> i64 {
    42
}
