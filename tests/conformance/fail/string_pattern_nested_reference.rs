pub fn probe() -> i64 {
    let nested = &"text";
    match nested { "text" => 1, _ => 0 }
}
