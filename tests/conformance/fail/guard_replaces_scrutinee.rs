pub fn probe() -> i64 {
    let mut value = Some(Box::new(1i64));
    match value { Some(_) if { value = None; true } => 10, _ => 42 }
}
