// expect: reject E_PRIVACY
// witness: {"rustAccept":false,"rustCode":"E0364"}
mod outer {
    pub mod inner { pub(in crate::outer) fn value() -> i64 { 42 } }
    pub(crate) use self::inner::value;
}
pub fn probe() -> i64 { 0 }
