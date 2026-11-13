fn unavailable() -> impl Iterator<Item = i64> { unreachable!() }
fn dependent() -> i64 { unavailable() }
fn independent() -> bool { 42i64 }
trait Items { type Item; }
fn malformed<I: Items<Item = bool>>(_input: I) { }
fn after() -> BoundaryMissing { 42i64 }
