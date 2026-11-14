pub fn probe() -> i64 {
    let mut values: Vec<i64> = Vec::new();
    let mut i: i64 = 0;
    while i < 1000 {
        values.push(i);
        i = i + 1;
    }
    values[999] + values.len() as i64
}
