trait Clone {}
trait Contract {
    fn take<T: crate::Clone>(&self, value: T) -> T;
}
mod nested {
    pub struct Counter;
    impl crate::Contract for Counter {
        fn take<T: Clone>(&self, value: T) -> T { value }
    }
}
pub fn probe() -> i64 { 0 }
