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
pub fn probe() -> i64 {
    require(Map {
        input: Once { value: Some(20) },
        function: |value: bool| if value { 42 } else { 0 },
    })
}
