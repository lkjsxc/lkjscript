pub fn probe() -> i64 {
    let values = vec![42i64];
    for value in &values {
        drop(values);
        return *value;
    }
    0
}
