use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::verify::{verify_plan, VerifiedMachinePlan};
use crate::{BackendLimits, NativeError};

mod append;
mod builder;
mod builder_core;
mod calls_control;
mod comparisons;
mod enum_heap_validity;
mod error;
mod heap;
mod heap_descriptor;
mod heap_validity;
mod identity;
mod ir;
mod machine;
mod numeric;
mod runtime;

pub use builder::FunctionBuilder;
pub use comparisons::*;
pub use error::PlanError;
pub use heap::{AllocationClass, HeapCallDescriptor, HeapOperation, StoreClass};
pub use identity::*;
pub use ir::FunctionPlan;
pub(crate) use ir::*;
pub use machine::{MachinePlanBuilder, StaticBytesIdentity};
pub use runtime::*;
