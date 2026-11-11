struct NoClone;
#[derive(Clone)]
struct Item<T> {
    value: T,
    missing: NoClone,
}
pub fn probe() -> i64 { 0 }
