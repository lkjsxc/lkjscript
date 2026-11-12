const K: i64 = 40;
static NAMES: [&str; 2] = ["a", "bc"];
pub fn probe() -> i64 { K + NAMES[1].len() as i64 }
