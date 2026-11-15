enum L { Nil, Cons(i64, Box<L>) }
fn len(l: &L) -> i64 { let mut n: i64 = 0; let mut c: &L = l; loop { match c { L::Nil => { return n; } L::Cons(_, t) => { n = n + 1; c = t; } } } }
pub fn probe() -> i64 { let l = L::Cons(1, Box::new(L::Cons(2, Box::new(L::Nil)))); len(&l) + 40 }
