#![allow(clippy::expect_used, clippy::field_reassign_with_default)]

mod canonical;

#[path = "jit_engines/allocation.rs"]
mod allocation;
#[path = "jit_engines/auto_tiering.rs"]
mod auto_tiering;
#[path = "jit_engines/buffer_boundaries.rs"]
mod buffer_boundaries;
#[path = "jit_engines/collector_free_scalar.rs"]
mod collector_free_scalar;
#[path = "jit_engines/reference_graphs.rs"]
mod reference_graphs;
#[path = "jit_engines/resource_island.rs"]
mod resource_island;
#[path = "jit_engines/resource_limits.rs"]
mod resource_limits;
#[path = "jit_engines/scalar_semantics.rs"]
mod scalar_semantics;
#[path = "jit_engines/scheduled_discovery.rs"]
mod scheduled_discovery;
#[path = "jit_engines/scheduled_kernels.rs"]
mod scheduled_kernels;
#[path = "jit_engines/tiering.rs"]
mod tiering;
#[path = "jit_engines/unique_island/mod.rs"]
mod unique_island;
