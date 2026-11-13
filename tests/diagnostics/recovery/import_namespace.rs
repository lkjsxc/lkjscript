mod uncertain {
    use missing::*;
    fn i64() {}
    fn dependent(value: i64) {}
}
fn generic<T>(value: T) {
    use missing::Other as T;
    let _: T = true;
}
fn independent(value: i64) -> i64 { value.absent() }
