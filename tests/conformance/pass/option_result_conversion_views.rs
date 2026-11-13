pub fn probe() -> i64 {
    let left = 40i64;
    let right = 2i64;
    let present: Option<&i64> = Some(&left);
    let ok: Result<&i64, &i64> = present.ok_or(&right);
    let absent: Option<&i64> = None;
    let error: Result<&i64, &i64> = absent.ok_or_else(|| &right);
    *ok.unwrap() + *error.unwrap_err()
}
