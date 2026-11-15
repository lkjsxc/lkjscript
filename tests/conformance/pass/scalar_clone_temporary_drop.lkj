// expect: value 42
// witness: {}
struct Owner<'a> { drops: &'a mut i64, value: u8 }
impl<'a> Drop for Owner<'a> {
    fn drop(&mut self) { *self.drops += 1i64; }
}
fn next(calls: &mut i64, value: u8) -> u8 { *calls += 1i64; value }
fn owner<'a>(drops: &'a mut i64, value: u8) -> Owner<'a> { Owner { drops, value } }
pub fn probe() -> i64 {
    let mut drops = 0i64;
    let mut calls = 0i64;
    let first = Owner { drops: &mut drops, value: next(&mut calls, 19u8) }.value.clone();
    assert!(drops == 1i64 && calls == 1i64);
    let second = owner(&mut drops, next(&mut calls, 21u8)).value.clone();
    assert!(drops == 2i64 && calls == 2i64);
    first as i64 + second as i64 + drops
}
