// expect: reject E_BORROW
// witness: {"rustAccept":false}
struct View<'a> { value: &'a i64, scalar: i64 }
fn wrong<'a>(carrier: &View<'a>) -> &'a i64 { &carrier.scalar }
pub fn probe() -> i64 { 0 }
