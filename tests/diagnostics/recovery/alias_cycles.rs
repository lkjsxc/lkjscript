type First = Second;
type Second = First;
fn unavailable(value: First) -> First { value }
fn independent(value: i64) -> i64 { value.absent() }
