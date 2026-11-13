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
fn exercise(which: i64) -> i64 {
    let mut value = if which == 0 { 41i64 } else { 42i64 };
    let choice = choose(&mut value, which);
    match choice {
        Choice::Held(reference) => {
            *reference = *reference + 1;
        }
        Choice::Empty => {}
        Choice::AlsoEmpty => {}
    }
    value
}
pub fn probe() -> i64 {
    exercise(0) + exercise(1) + exercise(2) - 84
}
