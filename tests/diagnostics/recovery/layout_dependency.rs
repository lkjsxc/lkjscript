struct Unavailable { field: Missing }
fn dependent(value: Unavailable) -> i64 { drop(value); 42 }
fn independent(value: Vec<i64>) -> i64 { value.unrelated() }
