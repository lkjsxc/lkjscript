trait Eligible {}
trait Extension {
    fn take(self) -> i64;
}
impl<T: Eligible> Extension for T {
    fn take(self) -> i64 {
        42
    }
}
struct Enabled {}
impl Eligible for Enabled {}
struct Plain {
    value: i64,
}
impl Plain {
    fn new() -> Plain {
        Plain { value: 42 }
    }
}
pub fn probe() -> i64 {
    assert_eq!(Plain::new().value, 42);
    let mut value = Some(42i64);
    assert_eq!(value.take().unwrap(), 42);
    Enabled {}.take()
}
