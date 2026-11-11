struct Bytes<'a> { bytes: &'a [u8] }
fn sum(view: Bytes<'_>) -> i64 {
    let mut result: i64 = 0;
    let mut i: usize = 0;
    while i < view.bytes.len() {
        result = result + view.bytes[i] as i64;
        i = i + 1;
    }
    result
}
pub fn probe() -> i64 {
    let bytes: [u8; 2] = [20, 22];
    sum(Bytes { bytes: &bytes })
}
