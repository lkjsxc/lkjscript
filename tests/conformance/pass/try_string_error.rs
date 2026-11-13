fn next(value: Option<i32>) -> Result<i32, String> {
    Ok(value.ok_or("missing")? + 1)
}
fn local_error(text: &str) -> Result<i32, String> {
    let error: Result<i32, &str> = Err(text);
    Ok(error?)
}
pub fn probe() -> i32 {
    let owned = "local error".to_string();
    match local_error(owned.as_str()) {
        Ok(_) => return 0,
        Err(error) => {
            if error != owned {
                return 1;
            }
        }
    }
    match next(None) {
        Ok(_) => 0,
        Err(error) => {
            if error == "missing".to_string() {
                next(Some(41)).unwrap()
            } else {
                2
            }
        }
    }
}
