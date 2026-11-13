trait Leaf {}
impl Leaf for i64 {}
struct Wrap<T> {
    value: T,
}
impl<T: Leaf> Leaf for Wrap<T> {}
trait Make {
    type Out: Leaf;
}
impl Make for bool {
    type Out = Wrap<Wrap<i64>>;
}
impl<T: Leaf> Make for Wrap<T> {
    type Out = Wrap<T>;
}
pub fn probe() -> i64 {
    42
}
