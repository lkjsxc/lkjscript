struct C { n: i64 }
impl C { fn new() -> C { C { n: 0 } } fn add(mut self, ir: i64) -> C { self.n += ir; self } }
pub fn probe() -> i64 { C::new().add(40).add(2).n }
