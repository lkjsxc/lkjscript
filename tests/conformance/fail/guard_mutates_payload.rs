pub fn probe() -> i64 { match Some(Box::new(1i64)) { Some(mut x) if { *x = 2; true } => *x, _ => 0 } }
