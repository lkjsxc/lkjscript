pub fn probe() -> i64 {
    let owned = Box::new(21i64);
    let consume = move || { let transferred = owned; *transferred };
    consume() + consume()
}
