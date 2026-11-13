struct Pair { a: i64, b: i64 }
fn field() -> i64 {
    let mut pair = Pair { a: 2, b: 0 };
    match pair {
        Pair { a: ref mut value, .. } if { pair.b = 40; *value > 0 } => *value + pair.b,
        _ => 0,
    }
}
fn wildcard() -> i64 {
    let mut value = Some(Box::new(1i64));
    match value { _ if { value = None; true } => 42, _ => 0 }
}
fn untested_shell() -> i64 {
    let mut pair = Pair { a: 0, b: 0 };
    match pair {
        Pair { a: _, .. } if { pair = Pair { a: 40, b: 2 }; true } => pair.a + pair.b,
        _ => 0,
    }
}
pub fn probe() -> i64 { field() + wildcard() + untested_shell() }
