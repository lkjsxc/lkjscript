// expect: value 42
use std::fmt::Write;
pub fn probe() -> i64 {
    let mut text = String::new();
    write!(&mut text, "{:04x}", 42u32).unwrap();
    assert!(text == "002a");
    assert!(format!("{value:>4}", value = 42i64) == "  42");
    42
}
