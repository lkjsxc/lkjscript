// Rust prints 42. Current check rejects E_BORROW because an inline closure's
// requested FnOnce kind does not yet guide its capture/body contract. The
// explicit reborrow is valid when this environment is consumed by the call.
// Binding this closure to a local before passing it to once is rejected by Rust.
fn once<B, F: FnOnce() -> B>(function: F) -> B {
    function()
}
pub fn probe() -> i64 {
    let mut value = 42i64;
    let reference = &mut value;
    let selected = once(move || &mut *reference);
    *selected
}
