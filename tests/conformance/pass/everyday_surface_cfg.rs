#[cfg(test)] mod nonexistent_test_module;
#[cfg(test)] fn unavailable() { this_function_does_not_exist(); }
#[cfg(any(test, not(unix)))] struct Missing { value: UnknownType }
#[cfg(all(not(test), target_os = "linux", target_arch = "x86_64"))]
fn number() -> i64 { 42 }
struct Counter;
impl Counter {
    #[cfg(test)] fn bad(&self) { unknown_method_body(); }
    fn good(&self) -> i64 { number() }
}
pub fn probe() -> i64 {
    #[cfg(test)] fn bad_local() { undefined(); }
    let value = Counter;
    value.good()
}
