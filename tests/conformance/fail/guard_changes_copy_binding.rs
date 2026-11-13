struct Pair { number: i64 }
pub fn probe() -> i64 {
    let mut pair = Pair { number: 1 };
    match pair { Pair { number } if { pair.number = 3; number > 0 } => number, _ => 42 }
}
