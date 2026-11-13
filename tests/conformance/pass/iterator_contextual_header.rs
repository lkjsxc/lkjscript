struct Source {
    value: i64,
}
trait Stream {
    type Item;
    fn read(self) -> Self::Item;
}
impl Stream for Source {
    type Item = i64;
    fn read(self) -> i64 {
        self.value
    }
}
trait Transform<I: Stream> {
    fn transform<B, F: FnOnce(I::Item) -> B>(self, function: F) -> B;
}
impl<I: Stream> Transform<I> for I {
    fn transform<B, F: FnOnce(I::Item) -> B>(self, function: F) -> B {
        function(self.read())
    }
}
pub fn probe() -> i64 {
    Source { value: 41 }.transform(|value| value + 1)
}
