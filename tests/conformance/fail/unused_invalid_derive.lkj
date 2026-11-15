// expect: reject E_TRAIT
// witness: {"rustAccept":false}
struct NoClone;
#[derive(Clone)]
struct Item<T> {
    value: T,
    missing: NoClone,
}
pub fn probe() -> i64 { 0 }
