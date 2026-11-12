enum T { Leaf(i64), Node(Box<T>, Box<T>) }
fn bump(t: &mut T) { match t { T::Leaf(v) => { *v += 1; } T::Node(l, r) => { bump(l); bump(r); } } }
fn sum(t: &T) -> i64 { match t { T::Leaf(v) => *v, T::Node(l, r) => sum(l) + sum(r) } }
pub fn probe() -> i64 { let mut t = T::Node(Box::new(T::Leaf(39)), Box::new(T::Leaf(1))); bump(&mut t); sum(&t) }
