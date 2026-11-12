pub fn probe() -> i64 { match Some(Box::new(1i64)) { Some(x) if { drop(x); false } => 1, _ => 0 } }
