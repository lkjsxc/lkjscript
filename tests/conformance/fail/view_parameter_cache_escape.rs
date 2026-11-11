struct View<'a> { value: &'a i64, cache: i64 }
fn leak<'a>(view: View<'a>) -> &'a i64 { &view.cache }
pub fn probe() -> i64 { let value: i64 = 42; *leak(View { value: &value, cache: 0 }) }
