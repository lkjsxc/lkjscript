struct View<'a> { value: &'a i64 }
fn leak<'a>(view: View<'a>) -> &'a &'a i64 { &view.value }
pub fn probe() -> i64 { let value: i64 = 42; **leak(View { value: &value }) }
