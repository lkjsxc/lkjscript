fn unavailable() -> Missing { unreachable!() }
fn caller() -> i64 { let _ = unavailable(); 42 }
fn independent() -> i64 { 42i64.absent_method() }
