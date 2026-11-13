mod values {
    pub struct Value { pub n: i64 }
    pub trait Read { fn read(&self) -> i64; }
    impl Read for Value { fn read(&self) -> i64 { self.n } }
}
fn local() -> i64 { use values::Read; let value=values::Value { n: 42 }; value.read() }
pub fn probe() -> i64 { let value=values::Value { n: 42 }; value.read() }
