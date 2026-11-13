fn shared(left: bool) -> i64 {
    let first: i64 = 20;
    let second: i64 = 22;
    let selected = if left { &first } else { &second };
    *selected
}
fn exclusive(left: bool) -> i64 {
    let mut first: i64 = 10;
    let mut second: i64 = 20;
    let selected = if left { &mut first } else { &mut second };
    *selected = *selected + 12;
    drop(selected);
    first + second
}
fn maybe<'a>(value: &'a i64, present: bool) -> Option<&'a i64> {
    if present { Some(value) } else { None }
}
fn optional(present: bool) -> i64 {
    let number: i64 = 42;
    let selected = maybe(&number, present);
    match selected { Some(value) => *value, None => 0 }
}
pub fn probe() -> i64 {
    shared(true) + shared(false) + exclusive(true) + exclusive(false)
        + optional(true) + optional(false) - 126
}
