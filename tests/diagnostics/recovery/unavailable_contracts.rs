fn unavailable(value: Missing) {}
const FIRST: i64 = SECOND;
const SECOND: i64 = FIRST;
fn uses_constant() -> i64 { FIRST }
fn uses_lifetime<'a>(value: &'a i64) -> &'a i64 { value }
type Global = &'static i64;
fn alias_lifetime(value: Global) -> i64 { *value }
fn local_lifetime() { let value = 42i64; let alias: &'static i64 = &value; }
fn independent() -> bool { 42i64 }
