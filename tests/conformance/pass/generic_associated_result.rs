trait Source {
    type Item;
    fn get(&self) -> Self::Item;
}
struct Integer { value: i64 }
impl Source for Integer {
    type Item = i64;
    fn get(&self) -> Self::Item { self.value }
}
fn read<T: Source>(value: &T) -> T::Item { value.get() }
pub fn probe() -> i64 { read(&Integer { value: 42 }) }
