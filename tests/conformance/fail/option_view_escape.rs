pub fn probe() -> i64 {
    let view = {
        let value = Some(42i64);
        value.as_ref()
    };
    *view.unwrap()
}
