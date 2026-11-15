// Rust runs42. Current check rejects E_BORROW: Option::as_mut returns a bounded
// reference to its payload, so a stored shared reference loses its exact origin.
// Direct pattern borrowing proves the same source program without this boundary.
struct Holder<'a> {
    value: &'a i64,
}
fn stored<'a>(owner: &mut Option<Holder<'a>>) -> Option<&'a i64> {
    match owner.as_mut() {
        Some(holder) => Some(holder.value),
        None => None,
    }
}
pub fn probe() -> i64 {
    let value = 42i64;
    let mut owner = Some(Holder { value: &value });
    let result = stored(&mut owner).unwrap();
    drop(owner);
    *result
}
