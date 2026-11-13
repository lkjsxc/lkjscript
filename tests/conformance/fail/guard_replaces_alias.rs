fn reset(value: &mut Option<i64>) { *value = None; }
pub fn probe() -> i64 {
    let mut value = Some(1i64);
    let pointer = &mut value;
    match *pointer { Some(_) if { reset(pointer); true } => 10, _ => 42 }
}
