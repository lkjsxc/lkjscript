pub fn probe() -> i64 {
    let mut sum: i64 = 0;
    for value in 0i64..10i64 {
        if value % 2 == 0 { continue; }
        sum = sum + value;
    }
    sum
}
