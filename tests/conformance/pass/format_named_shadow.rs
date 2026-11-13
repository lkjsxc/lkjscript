pub fn probe() -> i64 {
    let mut number = 20i64;
    let old = &number;
    let text = format!(
        "{old}",
        old = {
            number = 42;
            number
        }
    );
    assert!(text.len() == 2usize);
    number
}
