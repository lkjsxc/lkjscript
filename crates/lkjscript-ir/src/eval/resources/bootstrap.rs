use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use lkjscript_core::{ResourceOwnership, ResourceTable, ResourceTableLimits, ScopeId};

use super::*;

static NEXT_SCOPE: AtomicU64 = AtomicU64::new(1);

impl EvalResources {
    pub(crate) fn new(max_owned: usize) -> Result<Self, String> {
        let scope = next_scope().ok_or_else(|| "evaluator resource scope exhausted".to_owned())?;
        Self::with_scope(max_owned, scope)
    }

    pub(super) fn with_scope(max_owned: usize, scope: ScopeId) -> Result<Self, String> {
        let max_slots = max_owned
            .checked_add(2)
            .ok_or_else(|| "evaluator resource slot limit overflow".to_owned())?;
        let limits = ResourceTableLimits::new(
            max_slots,
            max_slots,
            max_owned,
            2,
            max_owned,
            NonZeroU64::MAX,
        )
        .map_err(|error| error.to_string())?;
        let mut table = ResourceTable::new(scope, limits);
        let mut providers = FakeProviders::new();
        let standard_input = install_borrowed(
            &mut table,
            &mut providers,
            lkjscript_core::ResourceKind::InputStream,
        )?;
        let standard_output = install_borrowed(
            &mut table,
            &mut providers,
            lkjscript_core::ResourceKind::OutputStream,
        )?;
        Ok(Self {
            table,
            standard_input: Some(standard_input),
            standard_output: Some(standard_output),
            #[cfg(test)]
            providers,
            metrics: EvalResourceMetrics {
                borrowed_installed: 2,
                ..EvalResourceMetrics::default()
            },
        })
    }
}

fn next_scope() -> Option<ScopeId> {
    let value = NEXT_SCOPE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .ok()?;
    ScopeId::new(value)
}

fn install_borrowed(
    table: &mut ResourceTable<FakeOwner>,
    providers: &mut FakeProviders,
    kind: lkjscript_core::ResourceKind,
) -> Result<EvalResource, String> {
    let provider = provider_for_kind(kind);
    let scope = table.scope();
    let payload = providers
        .borrowed(kind, provider, scope)
        .map_err(str::to_owned)?;
    let key = table
        .reserve_borrowed(kind, provider)
        .map_err(|error| error.to_string())?
        .commit(payload);
    Ok(EvalResource::new(
        key,
        kind,
        provider,
        scope,
        ResourceOwnership::Borrowed,
    ))
}
