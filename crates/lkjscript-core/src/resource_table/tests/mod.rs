#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::num::NonZeroU64;

use super::*;

mod access;
mod bounds;
mod cleanup;
mod lifecycle;

fn provider(value: u64) -> ProviderId {
    ProviderId::new(value).unwrap()
}

fn scope(value: u64) -> ScopeId {
    ScopeId::new(value).unwrap()
}

fn limits(slots: usize, max_generation: u64) -> ResourceTableLimits {
    ResourceTableLimits::new(
        slots,
        slots,
        slots,
        slots,
        slots,
        NonZeroU64::new(max_generation).unwrap(),
    )
    .unwrap()
}
