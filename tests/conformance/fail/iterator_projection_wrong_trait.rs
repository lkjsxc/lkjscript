mod numbers {
    pub trait Stream {
        type Item;
        fn read(self) -> Self::Item;
    }
}
mod flags {
    pub trait Stream {
        type Item;
        fn read(self) -> Self::Item;
    }
}
struct Source;
impl numbers::Stream for Source {
    type Item = i64;
    fn read(self) -> i64 {
        42
    }
}
impl flags::Stream for Source {
    type Item = bool;
    fn read(self) -> bool {
        true
    }
}
fn apply<I: numbers::Stream, F: FnOnce(I::Item) -> i64>(input: I, function: F) -> i64 {
    function(input.read())
}
pub fn probe() -> i64 {
    apply(Source, |value| if value { 42 } else { 0 })
}
