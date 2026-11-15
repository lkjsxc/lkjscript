trait A { fn f(&self) -> i64; }
struct X; struct Y;
impl A for X { fn f(&self) -> i64 { 40 } }
impl A for Y { fn f(&self) -> i64 { 2 } }
pub fn probe() -> i64 { let v: Vec<Box<dyn A>> = vec![Box::new(X), Box::new(Y)]; v[0].f() + v[1].f() }
