mod outer {
    pub mod inner {
        pub mod leaf {
            pub(in super::super) struct Token {
                pub(in super::super) value: i64,
            }
            impl Token {
                pub(in super::super) fn read(&self) -> i64 { self.value }
            }
            pub(in self) fn hidden() -> i64 { 2 }
            pub(in super::super::inner) fn increment() -> i64 { hidden() }
        }
        pub(in super) use self::leaf::Token;
        pub(in super) fn increment() -> i64 { leaf::increment() }
    }
    pub fn choose() -> i64 {
        let token = inner::Token { value: 40 };
        token.read() + inner::increment()
    }
}
pub fn probe() -> i64 { outer::choose() }
