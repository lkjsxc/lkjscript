fn apply<F: Fn() -> i64>(f:F)->i64 {f()} pub fn probe()->i64 {let mut x=0i64;apply(|| {x=x+1;x})}
