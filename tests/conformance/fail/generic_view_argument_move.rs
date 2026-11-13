fn identity<T>(value: T) -> T { value }
pub fn probe() -> i64 {
    let mut value = 42i64;
    let reference = &mut value;
    let moved = identity(reference);
    *reference + *moved
}
