trait Leaf {}
impl Leaf for i64 {}
struct Wrap<T> {
    value: T,
}
trait Make {
    type Out: Leaf;
}
impl<T> Make for Wrap<T> {
    type Out = T;
}
pub fn probe() -> i64 {
    42
}
