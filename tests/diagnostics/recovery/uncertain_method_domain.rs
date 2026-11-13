trait Extension { fn maybe(&self) -> i64; }
impl Extension for Missing {
    fn maybe(&self) -> i64 { 42 }
}
fn uncertain(value: i64) -> i64 { value.maybe() }
fn independent(value: i64) -> i64 { value.unrelated() }
