#![allow(clippy::expect_used, clippy::panic)]
mod evaluator;
pub(crate) mod fixtures;
mod ownership_aliases;
mod ownership_edges;
mod ownership_facts;
mod ownership_paths;
mod ownership_scale;
mod passes;
mod verification_cfg_scale;
mod verification_generics;
mod verification_region_product_scale;
mod verification_shape;
mod verification_traits;
