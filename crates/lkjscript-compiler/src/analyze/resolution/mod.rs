mod borrowing;
mod calls;
mod control;
mod derived_facts;
mod enums;
mod literals;
mod locals;
pub(super) mod matching;
mod model;
mod names;
mod products;
mod substitutions;
mod traits;

pub(in crate::analyze) use derived_facts::*;
