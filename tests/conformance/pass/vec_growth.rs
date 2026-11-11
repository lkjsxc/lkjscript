pub fn probe() -> i64 {
    let mut values: Vec<i64> = Vec::new();
    let mut i: usize = 0;
    while i < 1000 {
        values.push(i as i64);
        i = i + 1;
    }
    let mut total: i64 = 0;
    i = 0;
    while i < values.len() {
        total = total + values[i];
        i = i + 1;
    }
    total
}
