pub fn probe() -> i64 {
    let mut i: i64 = 0;
    let mut sum: i64 = 0;
    loop {
        i = i + 1;
        if i > 10 { break; }
        if i % 2 == 0 { continue; }
        sum = sum + i;
    }
    sum
}
