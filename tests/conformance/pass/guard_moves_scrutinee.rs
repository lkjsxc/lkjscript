pub fn probe() -> i64 {
    let value = Some(Box::new(1i64));
    match value { Some(_) if { drop(value); true } => 10, _ => 42 }
}
