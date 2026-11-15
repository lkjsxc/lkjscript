//! Closed gate roots: their complete emitted image excludes checker/frontend code.
pub mod diagnostic;
pub(crate) mod facts;
pub mod ir;
pub mod ir_text;
pub mod ir_binary;
mod semantic_tests {
    pub mod facts;
    pub mod ir_binary;
}

pub fn main() {
    assert_eq!(semantic_tests::facts::run() + semantic_tests::ir_binary::run(), 5usize);
}
