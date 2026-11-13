// Definition-site callable effect gap: a fixed named argument lifetime may be
// retained by the callable environment even when the result is a scalar.
// Rust rejects the assignment with E0506; lkjscript currently accepts this
// unused definition. The elided FnMut(&mut i64) form has a fresh call lifetime
// and Rust accepts that different contract.
fn invalid<'a, F: FnMut(&'a mut i64) -> i64>(mut function: F, value: &'a mut i64) -> i64 {
    let result = function(value);
    *value = 0;
    result
}

pub fn probe() -> i64 {
    42
}
