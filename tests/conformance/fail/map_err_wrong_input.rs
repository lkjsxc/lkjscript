pub fn probe() -> i64 {
    let input: Result<i64, String> = Err("x".to_string());
    let _result = input.map_err(|message: i64| message + 1);
    0
}
