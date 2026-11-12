struct C { n: i64 }
impl C { fn new() -> C { C { n: 0 } } fn add(mut self, k: i64) -> C { self.n += k; self } }
pub fn probe() -> i64 { C::new().add(40).add(2).n }
