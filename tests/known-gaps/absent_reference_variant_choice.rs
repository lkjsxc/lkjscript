pub fn probe() -> i64 {
    let mut value = 40i64;
    let captured = &mut value;
    let absent: Option<&mut i64> = None;
    let result: Result<&mut i64, &mut i64> = absent.ok_or_else(move || captured);
    match result {
        Ok(slot) | Err(slot) => { *slot = *slot + 2; }
    }
    value
}
