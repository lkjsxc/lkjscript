use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::verify::{verify_plan, VerifiedMachinePlan};
use crate::{BackendLimits, NativeError};

mod builder;
mod comparisons;
mod error;
mod heap;
mod heap_descriptor;
mod heap_validity;
mod identity;
mod ir;
mod machine;
mod numeric;
mod runtime;
mod structural;
mod value_type;

pub use builder::FunctionBuilder;
pub use comparisons::*;
pub use error::PlanError;
pub use heap::{AllocationClass, HeapCallDescriptor, HeapOperation, StoreClass};
pub use identity::*;
pub(crate) use ir::*;
pub use ir::{FailureCleanupCall, FunctionPlan};
pub use machine::{MachinePlanBuilder, StaticBytesIdentity};
pub use runtime::*;
pub use structural::*;
pub use value_type::*;
