trait Next { type Item; fn next(&mut self) -> Option<Self::Item>; }
struct Iter<'a, T> { items: &'a [T], index: usize }
impl<'a, T> Next for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        if self.index < self.items.len() {
            let index = self.index;
            self.index = index + 1;
            Some(&self.items[index])
        } else { None }
    }
}

struct Map<I, F> { iter: I, f: F }
impl<I: Next, F: FnMut(I::Item) -> i64> Next for Map<I, F> {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        match self.iter.next() { Some(value) => Some((self.f)(value)), None => None }
    }
}
fn mapped<I: Next, F: FnMut(I::Item) -> i64>(iter: I, f: F) -> Map<I,F> { Map { iter, f } }
pub fn probe() -> i64 {
    let items = [20i64, 22i64];
    let iter = Iter { items: items.as_slice(), index: 0 };
    let mut mapped = mapped(iter, |value: &i64| *value);
    let mut total=0i64;
    while let Some(value) = mapped.next() { total = total + value; }
    total
}
