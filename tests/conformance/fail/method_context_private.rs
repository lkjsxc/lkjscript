mod hidden {
    pub struct Wrapper<T> { value: T }
    impl<T> Wrapper<T> {
        fn apply<F: FnOnce(T) -> i64>(self, function: F) -> i64 { function(self.value) }
    }
    pub fn value() -> Wrapper<i64> { Wrapper { value: 41 } }
}
pub fn probe() -> i64 { hidden::value().apply(|value| value + 1) }
