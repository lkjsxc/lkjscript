trait Pick { fn pick<'a>(&self, a: &'a i64, b: &'a i64) -> &'a i64; }
struct L; 
impl Pick for L { fn pick<'a>(&self, a: &'a i64, b: &'a i64) -> &'a i64 { a } }
pub fn probe() -> i64 {
    let x: i64 = 1; let l = L;
    let r: &i64;
    { let y: i64 = 2; r = l.pick(&x, &y); }
    *r
}
