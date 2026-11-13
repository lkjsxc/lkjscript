pub fn probe() -> i64 {
    let mut value = 1i64;
    match value { _ if { value = 2; false } => 0, 2 => 2, _ => 42 }
}
