enum Choice<'a> {
    Held(&'a mut i64),
    Empty,
    AlsoEmpty,
}
fn choose<'a>(value: &'a mut i64, which: i64) -> Choice<'a> {
    if which == 0 {
        Choice::Held(value)
    } else if which == 1 {
        Choice::Empty
    } else {
        Choice::AlsoEmpty
    }
}
fn invalid(which: i64) -> i64 {
    let mut value = 42i64;
    let choice = choose(&mut value, which);
    match choice {
        Choice::Held(_) => {}
        Choice::Empty => {}
        Choice::AlsoEmpty => {}
    }
    let result = value;
    drop(choice);
    result
}
pub fn probe() -> i64 {
    invalid(0)
}
