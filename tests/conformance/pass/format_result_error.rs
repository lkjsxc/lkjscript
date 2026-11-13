use std::fmt::{self, Write};
struct Bad;
impl fmt::Display for Bad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("!")?;
        Err(fmt::Error)
    }
}
pub fn probe() -> i64 {
    let mut text = String::new();
    let result = write!(&mut text, "before{}after", Bad);
    assert!(result.is_err());
    assert_text(text.as_str(), "before!");
    42
}
fn assert_text(actual: &str, expected: &str) {
    let a = actual.as_bytes();
    let b = expected.as_bytes();
    assert!(a.len() == b.len());
    let mut index = 0usize;
    while index < a.len() {
        assert!(a[index] == b[index]);
        index = index + 1;
    }
}
