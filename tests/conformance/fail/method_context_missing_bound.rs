trait Required {}
struct Wrapper<T> { value: T }
impl<T: Required> Wrapper<T> {
    fn apply<F: FnOnce(T) -> i64>(self, function: F) -> i64 { function(self.value) }
}
pub fn probe() -> i64 { Wrapper { value: 42i64 }.apply(|value: i64| value) }
