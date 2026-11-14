pub fn argument(arguments: &[Vec<u8>], index: usize, fallback: u64) -> u64 {
    if index >= arguments.len() {
        return fallback;
    }
    let mut value = 0u64;
    let mut at = 0usize;
    while at < arguments[index].len() {
        let byte = arguments[index][at];
        assert!(byte >= 48u8 && byte <= 57u8);
        value = value * 10u64 + (byte - 48u8) as u64;
        at = at + 1usize;
    }
    value
}

pub fn decimal(mut value: u64) -> Vec<u8> {
    let mut reversed = Vec::new();
    loop {
        reversed.push((value % 10u64) as u8 + 48u8);
        value = value / 10u64;
        if value == 0u64 {
            break;
        }
    }
    let mut bytes = Vec::new();
    while reversed.len() != 0usize {
        bytes.push(reversed.pop().unwrap());
    }
    bytes.push(10u8);
    bytes
}
