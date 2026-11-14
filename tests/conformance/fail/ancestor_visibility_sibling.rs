// expect: reject E_PRIVACY
// witness: {"rustAccept":false,"rustCode":"E0742"}
mod outer {
    pub mod sibling {}
    pub mod inner { pub(in crate::outer::sibling) fn value() -> i64 { 42 } }
}
pub fn probe() -> i64 { 0 }
