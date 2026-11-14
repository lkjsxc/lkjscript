// expect: value 42
pub fn probe() -> i64 {
    let text = include_str!("include.txt");
    let bytes = include_bytes!("include.txt");
    assert!(text == "42\n");
    assert!(bytes.len() == 3 && bytes[0] == 52u8 && bytes[1] == 50u8);
    text[..2].parse::<i64>().unwrap()
}
