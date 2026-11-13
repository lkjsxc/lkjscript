mod left {
    pub trait Leaf {}
}
mod right {
    pub trait Leaf {}
}
struct Element {}
impl right::Leaf for Element {}
trait Make {
    type Out: left::Leaf;
}
impl Make for bool {
    type Out = Element;
}
pub fn probe() -> i64 {
    42
}
