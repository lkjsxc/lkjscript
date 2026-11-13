mod outer {
    pub mod inner {}
    pub(in self::inner) fn value() -> i64 { 42 }
}
pub fn probe() -> i64 { 0 }
