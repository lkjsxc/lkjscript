// expect: reject E_PRIVACY
// witness: {"rustAccept":false,"rustCode":"E0742"}
mod outer {
    pub mod inner {}
    pub(in self::inner) fn value() -> i64 { 42 }
}
pub fn probe() -> i64 { 0 }
