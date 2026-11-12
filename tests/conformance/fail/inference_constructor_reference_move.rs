fn invalid() {
    let mut value: i64 = 1;
    let reference = &mut value;
    let held = Some(reference);
    drop(held);
    *reference = 3;
}
