fn invalid() {
    let mut value: i64 = 1;
    let reference = &mut value;
    let moved = reference;
    *moved = 2;
    *reference = 3;
}
