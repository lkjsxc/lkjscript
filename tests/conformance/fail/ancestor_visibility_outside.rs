// expect: reject E_PRIVACY
// witness: {"rustAccept":false,"rustCode":"E0603"}
mod outer {
    pub mod inner {
        pub mod leaf { pub(in super::super) fn value() -> i64 { 42 } }
    }
}
pub fn probe() -> i64 { outer::inner::leaf::value() }
