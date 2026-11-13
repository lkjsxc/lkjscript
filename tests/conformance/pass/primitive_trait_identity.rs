trait Value {
    fn value(&self) -> i64;
}
impl Value for u64 {
    fn value(&self) -> i64 {
        20
    }
}
impl Value for usize {
    fn value(&self) -> i64 {
        22
    }
}
fn read<T: Value>(value: T) -> i64 {
    value.value()
}
pub fn probe() -> i64 {
    read(1u64) + read(1usize)
}
