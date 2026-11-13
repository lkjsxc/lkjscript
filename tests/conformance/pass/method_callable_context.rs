struct Wrapper<T> {
    value: T,
}
impl<T> Wrapper<T> {
    fn transform<U, F: FnOnce(T) -> U>(self, function: F) -> U {
        function(self.value)
    }
    fn inspect<U, F: FnOnce(&T) -> U>(&self, function: F) -> U {
        function(&self.value)
    }
}
type Alias<T> = Wrapper<T>;
pub fn probe() -> i64 {
    let wrapped = Alias { value: 41i64 };
    let reference = &wrapped;
    assert!(reference.inspect(|value| *value + 1) == 42);
    assert!(wrapped.transform(|value| value + 1) == 42);
    let owned = Wrapper { value: Box::new(42i64) };
    assert!(owned.inspect(|value| **value) == 42);
    let value: Option<i64> = Some(41);
    let result = value.map(|input| Some(input + 1)).and_then(|input| input);
    let absent: Option<i64> = None;
    assert!(absent.or_else(|| None).is_none());
    let result_value: Result<i64, i64> = Ok(41);
    assert!(result_value.and_then(|input| Ok(input + 1)).unwrap() == 42);
    result.unwrap_or(0)
}
