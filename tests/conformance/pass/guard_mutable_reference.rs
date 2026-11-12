pub fn probe() -> i64 {
    let mut value = 1i64;
    match value { ref mut reference if *reference > 0 => { *reference = 2; }, _ => {} }
    value
}
