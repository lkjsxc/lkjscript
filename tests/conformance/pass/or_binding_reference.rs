pub fn probe() -> i64 { let (true | false, ref value) = (true, Box::new(42i64)); **value }
