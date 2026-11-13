mod first { pub fn read() -> i64 { 1 } }
mod second { pub fn read() -> i64 { 22 } }
use first::*;
use second::read;
mod local {
    use super::first::*;
    use super::second::*;
    fn read() -> i64 { 20 }
    pub fn value() -> i64 { read() }
}
pub fn probe() -> i64 { read() + local::value() }
