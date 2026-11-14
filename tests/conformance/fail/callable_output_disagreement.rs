// expect: reject E_TYPE
// witness: {"rustCode":"E0308"}
fn apply<B, F: FnOnce() -> B>(function: F, witness: B) -> B {
    drop(witness);
    function()
}
pub fn probe() -> i64 {
    if apply(|| 42i64, true) { 42 } else { 0 }
}
