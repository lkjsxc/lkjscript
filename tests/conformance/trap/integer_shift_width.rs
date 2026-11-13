fn shift(count: u16) -> u8 {
    1u8 << count
}
pub fn probe() -> u8 {
    shift(8)
}
