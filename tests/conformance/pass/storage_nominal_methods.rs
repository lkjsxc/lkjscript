// expect: value 42
// entry: main
// witness: {}
struct Counter { count: usize }
impl Counter {
    fn truncate(&mut self, count: usize) { self.count = count; }
    fn sync(&mut self) -> usize { self.count }
}
type Ordinary = Counter;
pub fn main() -> i64 {
    let mut value = Ordinary { count: 0usize };
    value.truncate(42usize);
    value.sync() as i64
}
