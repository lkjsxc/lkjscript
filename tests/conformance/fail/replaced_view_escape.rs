struct View<'a> { value: &'a i64 }
fn leak<'a>(external: &'a i64) -> View<'a> {
    let local: i64 = 7;
    let mut view = View { value: &local };
    std::mem::replace(&mut view, View { value: external })
}
pub fn probe() -> i64 { let external: i64 = 42; *leak(&external).value }
