trait Mapper {
    fn take<T>(&self, value: T) -> T;
}
struct Identity {}
impl Mapper for Identity {
    fn take<T>(&self, value: T) -> T { value }
}
pub fn probe() -> i64 { Identity {}.take::<bool>(42i64) }
