// expect: reject E_TRAIT
// witness: {"rustCode":"E0275"}
trait A {}
trait B {}
impl<T: B> A for T {}
impl<T: A> B for T {}
trait Make {
    type Out: A;
}
impl Make for bool {
    type Out = i64;
}
pub fn probe() -> i64 {
    42
}
