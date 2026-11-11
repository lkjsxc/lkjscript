struct Guard { value: i64 }
impl Drop for Guard { fn drop(&mut self) {} }
pub fn probe() -> i64 {
    let mut guard = Guard { value: 42 };
    guard.drop();
    guard.value
}
