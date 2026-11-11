pub fn probe() -> i64 {
    let bytes = "immutable".as_bytes();
    bytes[0] = 65u8;
    bytes[0] as i64
}
