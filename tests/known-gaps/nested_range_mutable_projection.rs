// Rust accepts this and returns 42. The direct projection without `[..]` works.
// The mutable slice primitive does not yet promote the compiler-created range
// borrow, so this currently reports E_BORROW: mutable slice of immutable place.
pub fn probe() -> i64 {
    let mut values = [vec![40i64]];
    values[..][0].as_mut_slice()[0] += 2;
    values[0][0]
}
