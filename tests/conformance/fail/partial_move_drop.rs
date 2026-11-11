struct Guard { value: Box<i64> }
impl Drop for Guard { fn drop(&mut self) {} }
pub fn probe() -> i64 {
    let guard = Guard { value: Box::new(42i64) };
    let value = guard.value;
    *value
}
