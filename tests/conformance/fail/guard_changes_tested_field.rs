struct Pair { number: i64, item: Option<i64> }
pub fn probe() -> i64 {
    let mut pair = Pair { number: 1, item: Some(2) };
    match pair { Pair { item: Some(_), .. } if { pair.item = None; true } => 10, _ => 42 }
}
