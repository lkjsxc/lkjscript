pub fn probe() -> i64 {
    let values = vec![42i64];
    let item = values.get(0usize);
    let copied = item;
    drop(item);
    let answer = match copied { Some(value) => *value, None => 0i64 };
    drop(values);
    answer
}
