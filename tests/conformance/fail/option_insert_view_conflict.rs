pub fn probe() -> i64 {
    let mut value = Some(Box::new(20i64));
    let view = value.get_or_insert(Box::new(22));
    let old = value.take();
    **view
}
