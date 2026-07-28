#![allow(clippy::expect_used)]

use lkjscript_core::{
    CapabilityKind, ProviderId, ResourceKind, ResourceOwnership, ResourceTableError, ScopeId,
};

use super::*;

fn scope(value: u64) -> ScopeId {
    ScopeId::new(value).expect("test scope is nonzero")
}

fn session(value: u64) -> EvalResources {
    EvalResources::with_scope(32, scope(value)).expect("create evaluator resource session")
}

fn assert_exact_access(resources: &mut EvalResources, resource: &EvalResource) {
    resources
        .access_binding(
            resource,
            resource.kind,
            resource.provider,
            resource.ownership,
        )
        .expect("exact resource binding must resolve");
}

mod lifecycle;
mod rejection;
mod teardown;
