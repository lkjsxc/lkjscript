// expect: reject E_PRIVACY
// witness: {"rustAccept":false,"rustCode":"E0624"}
mod outer {
    pub mod inner {
        pub struct Token;
        impl Token { pub(in crate::outer) fn value(&self) -> i64 { 42 } }
    }
}
pub fn probe() -> i64 { outer::inner::Token.value() }
