// expect: trap R0003
pub fn probe() -> usize {
    let text = "é";
    text[1..].len()
}
