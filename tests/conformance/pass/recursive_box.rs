enum List { End, Next(i64, Box<List>) }
fn sum(list: &List) -> i64 {
    match list { List::End => 0, List::Next(value, next) => *value + sum(next) }
}
pub fn probe() -> i64 {
    let list = List::Next(20, Box::new(List::Next(22, Box::new(List::End))));
    sum(&list)
}
