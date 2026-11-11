struct Ticket { value: i64 }
pub fn probe() -> i64 {
    let first = Ticket { value: 21 };
    let second = first;
    first.value + second.value
}
