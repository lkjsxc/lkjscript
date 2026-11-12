pub fn probe() -> i64 {
    let mut value = 1i64;
    let reference = &mut value;
    let moved = reference;
    drop(moved);
    std::mem::replace(reference, 2)
}
