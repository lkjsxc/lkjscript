// expect: reject E_PRIVACY
// witness: {"rustAccept":false,"rustCodes":["E0433","E0578"]}
pub mod outer {
    use crate::outer as alias;
    pub mod inner { pub(in crate::outer::alias) fn value() -> i64 { 42 } }
}
pub fn probe() -> i64 { 0 }
