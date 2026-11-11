#[derive(Copy, Clone)]
struct View<'a> { value: &'a i64 }
fn leak<'a>(external: &'a i64) -> View<'a> {
    let local: i64 = 7;
    let view = View { value: &local };
    view.clone()
}
pub fn probe() -> i64 { let external: i64 = 42; *leak(&external).value }
