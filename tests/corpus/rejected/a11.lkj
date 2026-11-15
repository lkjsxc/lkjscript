struct W<T> { t: T }
fn wrap<T>(t: T) -> W<T> { W { t } }
pub fn probe() -> i64 {
    let w: W<&i64>;
    { let y: i64 = 5; w = wrap(&y); }
    *w.t
}
