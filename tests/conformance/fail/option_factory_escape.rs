pub fn probe() -> i64 {
    let result: Result<(), &i64>;
    {
        let value = 42i64;
        let absent: Option<()> = None;
        result = absent.ok_or_else(|| &value);
    }
    *result.unwrap_err()
}
