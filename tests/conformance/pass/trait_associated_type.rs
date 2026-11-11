trait Source { type Item; fn get(&self) -> Self::Item; }
struct Integer { value: i64 }
impl Source for Integer {
    type Item = i64;
    fn get(&self) -> i64 { self.value }
}
pub fn probe() -> i64 { Integer { value: 42 }.get() }
