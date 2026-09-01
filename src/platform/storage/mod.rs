//! Generic immutable object and packed physical storage under Graph 6 construction.

pub(crate) mod catalog;
pub(crate) mod contract;
pub(crate) mod directory;
pub(crate) mod memory;
pub(crate) mod object;
pub(crate) mod pack;
pub(crate) mod page_store;

#[cfg(test)]
mod tests;
