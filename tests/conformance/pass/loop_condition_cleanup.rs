pub fn probe() -> i64 {
    let values = vec![vec![20i64, 22i64]];
    let mut index: usize = 0;
    let mut total: i64 = 0;
    while index < values[0].len() {
        total = total + values[0][index];
        index = index + 1;
    }
    total
}
