mod debug;
mod heap_object;
mod model;

pub use heap_object::HeapObj;
pub use lkjscript_contracts::{CapabilityKind, ResourceKind};
pub use model::Value;

#[cfg(test)]
mod tests;
