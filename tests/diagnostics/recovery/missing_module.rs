mod missing;
fn wrong_type() -> bool { 42i64 }
fn method(value: i64) -> i64 { value.absent() }
fn name() { missing::unknown(); }
