struct String {
    bytes: Vec<u8>,
}
impl String {
    fn as_str(&self) -> &str {
        "abc"
    }
}
pub fn probe() -> i64 {
    let value = String {
        bytes: vec![1u8, 2, 3],
    };
    let part = &value[..];
    part.len() as i64
}
