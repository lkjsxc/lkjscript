enum Answer { Missing, Found(i64) }
fn value(answer: &Answer) -> i64 {
    match answer { Answer::Missing => 0, Answer::Found(value) => *value }
}
pub fn probe() -> i64 {
    let answer = Answer::Found(21);
    value(&answer) + value(&answer)
}
