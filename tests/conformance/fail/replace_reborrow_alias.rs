pub fn probe() -> i64 {
    let mut value = 1i64;
    let reference = &mut value;
    let shared = &*reference;
    let old = std::mem::replace(reference, 2);
    old + *shared
}
