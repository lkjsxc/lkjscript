const IR: i64 = 40;
static NAMES: [&str; 2] = ["a", "bc"];
pub fn probe() -> i64 { IR + NAMES[1].len() as i64 }
