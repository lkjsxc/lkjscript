pub fn probe() -> i64 {
    let mut sequence: i64 = 0;
    for mut value in 1i64..4i64 {
        sequence = sequence * 10 + value;
        value = 100;
        sequence = sequence + value - 100;
    }
    sequence
}
