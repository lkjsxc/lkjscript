// Rust accepts; the current definition verifier explicitly rejects E_UNSUPPORTED because an opaque callable may replace stored reference origins through this mutable receiver. The mutable stored-origin contract is not implemented.
fn test<'a, F: FnMut(&mut &'a mut i64)>(mut value: &'a mut i64, mut f: F) {
    f(&mut value);
    *value = 42;
}
pub fn probe() -> i64 {
    42
}
