mod first {
    pub struct Value { pub n: i64 }
    pub trait Read { fn read(&self) -> i64; }
    impl Read for Value { fn read(&self) -> i64 { self.n } }
}
mod second {
    pub struct Value { pub n: i64 }
    pub trait Read { fn read(&self) -> i64; }
    impl Read for Value { fn read(&self) -> i64 { self.n } }
}
fn first_value() -> i64 { use first::Read; let value=first::Value { n: 20 }; value.read() }
fn second_value() -> i64 { use second::Read; let value=second::Value { n: 22 }; value.read() }
pub fn probe() -> i64 { first_value() + second_value() }
