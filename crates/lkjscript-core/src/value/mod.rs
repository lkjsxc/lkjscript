mod heap_object;
mod model;

pub use heap_object::HeapObj;
pub use lkjscript_contracts::CapabilityKind;
pub use model::{Value, MAX_SMALL_I64, MIN_SMALL_I64};

#[cfg(test)]
mod tests;
