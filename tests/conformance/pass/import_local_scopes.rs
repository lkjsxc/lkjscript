mod first { pub fn answer() -> i64 { 20 } }
mod second { pub fn answer() -> i64 { 22 } }
fn value() -> i64 { 99 }
fn local() -> i64 {
    let left = value();
    use first::answer as value;
    let right = { use second::answer as value; value() };
    assert!(self::value() == 99);
    left + right
}
fn outside() -> i64 { value() }
pub fn probe() -> i64 { assert!(outside() == 99); local() }
