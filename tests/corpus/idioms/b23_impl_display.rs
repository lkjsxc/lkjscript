use std::fmt;
struct P { x: i64 }
impl fmt::Display for P { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.x) } }
pub fn probe() -> i64 { let p = P { x: 42 }; format!("{}", p).len() as i64 + 40 }
