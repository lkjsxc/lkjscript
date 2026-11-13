trait Next {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
struct Once {
    value: Option<i64>,
}
impl Next for Once {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        self.value.take()
    }
}
struct Map<I, F> {
    input: I,
    function: F,
}
impl<I: Next, F: FnMut(I::Item) -> B, B> Next for Map<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        match self.input.next() {
            Some(value) => Some((self.function)(value)),
            None => None,
        }
    }
}
fn require<I: Next>(mut input: I) -> i64 {
    if input.next().is_some() { 42 } else { 0 }
}
trait Collection {
    type IntoIter: Next;
    fn into_iter(self) -> Self::IntoIter;
}
impl<I: Next, F: FnMut(I::Item) -> B, B> Collection for Map<I, F> {
    type IntoIter = Map<I, F>;
    fn into_iter(self) -> Map<I, F> {
        self
    }
}
fn through<C: Collection>(value: C) -> i64 {
    require(value.into_iter())
}
pub fn probe() -> i64 {
    through(Map {
        input: Once { value: Some(20) },
        function: |value: i64| value + 22,
    })
}
