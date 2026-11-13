mod outer {
    pub mod inner {
        pub struct Token { pub(in crate::outer) value: i64 }
        pub fn token() -> Token { Token { value: 42 } }
    }
}
pub fn probe() -> i64 { outer::inner::token().value }
