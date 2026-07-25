mod outcomes;
mod programs;

pub use outcomes::{evaluator, execution, Scalar};
pub use programs::{compile, f64_loop, forced};
