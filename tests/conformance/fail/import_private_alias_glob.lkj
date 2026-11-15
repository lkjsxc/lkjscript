// expect: reject E_PRIVACY
// witness: {"rustAccept":false,"rustCode":"E0603"}
pub struct Foo{pub n:i64} mod outer{use super::Foo as Private;pub mod child{pub use super::*;}} pub fn probe()->i64{let v=outer::child::Private{n:42};v.n}
