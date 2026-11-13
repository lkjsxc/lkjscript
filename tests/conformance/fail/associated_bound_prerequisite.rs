trait Leaf {}
struct Wrap<T> {
    value: T,
}
impl<T: Leaf> Leaf for Wrap<T> {}
trait Make {
    type Out: Leaf;
}
impl<T> Make for Wrap<T> {
    type Out = Wrap<T>;
}
pub fn probe() -> i64 {
    42
}
