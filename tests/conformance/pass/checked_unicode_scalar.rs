pub fn probe() -> i64 {
    let values = [0u32, 0x7fu32, 0x80u32, 0xd7ffu32, 0xe000u32, 0x10ffffu32];
    let mut index = 0usize;
    while index < values.len() {
        let code = values[index];
        let character = char::from_u32(code).unwrap();
        if character as u32 != code { return 0; }
        index += 1;
    }
    if char::from_u32(0xd800u32).is_some() || char::from_u32(0xdfffu32).is_some()
        || char::from_u32(0x110000u32).is_some() || char::from_u32(0xffffffffu32).is_some() { return 0; }
    42
}
