// Rust accepts: next moves the stored item out of the iterator. The generic
// adapter currently retains the receiver as a possible Item origin.
struct One<'a> {
    value: Option<&'a mut i64>,
}
impl<'a> Iterator for One<'a> {
    type Item = &'a mut i64;
    fn next(&mut self) -> Option<Self::Item> {
        self.value.take()
    }
}
pub fn probe() -> i64 {
    let mut value = 40i64;
    let mut iterator = One {
        value: Some(&mut value),
    }
    .map(|item| item);
    let selected = iterator.next().unwrap();
    drop(iterator);
    *selected = *selected + 2;
    value
}
