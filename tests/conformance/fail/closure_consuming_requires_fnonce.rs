fn apply<F: FnMut() -> i64>(mut f:F)->i64 {f()} pub fn probe()->i64 {let x=Box::new(42i64);apply(move|| {let y=x;*y})}
