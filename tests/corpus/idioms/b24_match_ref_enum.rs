enum T { Leaf(i64), Node(Box<T>, Box<T>) }
fn sum(t: &T) -> i64 { match t { T::Leaf(v) => *v, T::Node(l, r) => sum(l) + sum(r) } }
pub fn probe() -> i64 { let t = T::Node(Box::new(T::Leaf(40)), Box::new(T::Leaf(2))); sum(&t) }
