struct String {
    value: i32,
}
pub fn probe() -> Result<i32, String> {
    let error: Result<i32, &str> = Err("no conversion");
    Ok(error?)
}
