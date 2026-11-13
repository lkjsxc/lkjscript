use unavailable::Maybe;
struct S;
trait Visible {
    fn value(&self) -> i64;
}
impl Visible for S {
    fn value(&self) -> i64 {
        42
    }
}
mod hidden {
    pub trait Maybe {
        fn value(&self) -> bool;
    }
    impl Maybe for super::S {
        fn value(&self) -> bool {
            true
        }
    }
}
fn dependent() -> bool {
    S.value()
}
fn independent() -> bool {
    42i64
}
