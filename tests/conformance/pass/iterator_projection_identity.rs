mod numbers {
    pub trait Stream {
        type Item;
        fn value(self) -> Self::Item;
    }
}
mod flags {
    pub trait Stream {
        type Item;
        fn value(self) -> Self::Item;
    }
}
struct Source;
type Alias = Source;
impl numbers::Stream for Source {
    type Item = i64;
    fn value(self) -> i64 {
        41
    }
}
impl flags::Stream for Source {
    type Item = bool;
    fn value(self) -> bool {
        true
    }
}
fn number<I: numbers::Stream, F: FnOnce(I::Item) -> i64>(input: I, function: F) -> i64 {
    function(input.value())
}
fn flag<I: flags::Stream, F: FnOnce(I::Item) -> i64>(input: I, function: F) -> i64 {
    function(input.value())
}
pub fn probe() -> i64 {
    let source: Alias = Source;
    assert_eq!(number(source, |value| value + 1), 42);
    flag(Source, |value| if value { 42 } else { 0 })
}
