const BASE: i64 = 20;
mod outer {
    use super::*;
    const EXTRA: i64 = 22;
    pub mod inner {
        use super::super::*;
        pub fn base() -> i64 { BASE }
    }
    pub fn value() -> i64 { inner::base() + EXTRA }
}
pub fn probe() -> i64 { outer::value() }
