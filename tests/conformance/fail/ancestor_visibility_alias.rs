pub mod outer {
    use crate::outer as alias;
    pub mod inner { pub(in crate::outer::alias) fn value() -> i64 { 42 } }
}
pub fn probe() -> i64 { 0 }
