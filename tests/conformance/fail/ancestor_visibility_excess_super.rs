mod outer {
    pub mod inner { pub(in super::super::super) fn value() -> i64 { 42 } }
}
pub fn probe() -> i64 { 0 }
