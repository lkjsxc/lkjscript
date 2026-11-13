fn bad<T>(x:T) { let f=move||drop(x); f();f(); } pub fn probe()->i64 {bad(1i64);42}
