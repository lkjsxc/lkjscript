fn hidden()->i64{42} pub mod child { pub use super::*; pub fn value()->i64 { hidden() } } pub fn probe()->i64{child::value()}
