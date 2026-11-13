trait Required {}
trait Alternative {
    fn apply<F: FnOnce(bool) -> i64>(self, function: F) -> i64;
}
struct Wrapper<T> {
    value: T,
}
impl<T: Required> Wrapper<T> {
    fn apply<F: FnOnce(T) -> i64>(self, function: F) -> i64 {
        function(self.value)
    }
}
impl Alternative for Wrapper<i64> {
    fn apply<F: FnOnce(bool) -> i64>(self, function: F) -> i64 {
        function(true)
    }
}
pub fn probe() -> i64 {
    Wrapper { value: 0i64 }.apply(|flag| if flag { 42i64 } else { 0i64 })
}
