#![allow(clippy::expect_used, clippy::field_reassign_with_default)]

mod canonical;

#[path = "jit_engines/allocation.rs"]
mod allocation;
#[path = "jit_engines/auto_tiering.rs"]
mod auto_tiering;
#[path = "jit_engines/buffer_boundaries.rs"]
mod buffer_boundaries;
#[path = "jit_engines/reference_graphs.rs"]
mod reference_graphs;
#[path = "jit_engines/resource_limits.rs"]
mod resource_limits;
#[path = "jit_engines/scalar_semantics.rs"]
mod scalar_semantics;
#[path = "jit_engines/tiering.rs"]
mod tiering;
