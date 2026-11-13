// Rust prints 42. Current check rejects E_BORROW because the solved reference type
// inside Option<_> is treated as a coercion site. The underscore only infers a
// type: returning the captured descriptor should move it and infer FnOnce.
// A future fix must retain coercions at explicit positions such as Option<&mut _>.
pub fn probe() -> i64 {
    let mut value = 42i64;
    let reference = &mut value;
    let once = move || -> Option<_> { Some(reference) };
    *once().unwrap()
}
