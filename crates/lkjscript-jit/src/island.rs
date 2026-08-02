use std::num::NonZeroU64;

use crate::*;
use lkjscript_core::{ProviderId, ResourceKey, ResourceTable, ResourceTableLimits, ScopeId};
use lkjscript_executable::NativeServiceError;
use lkjscript_native::{CapabilityKind, NativeResource, NativeUnique, ResourceKind};

mod list_equality;
mod list_values;
mod lists;
mod services;
mod structural;
mod unique;
use structural::JitStructuralRuntime;
use unique::JitUniqueRuntime;

struct BorrowedStandardInput;

pub(crate) struct JitIslandServices {
    table: ResourceTable<BorrowedStandardInput>,
    stdin: Option<ResourceKey>,
    provider: ProviderId,
    scope: ScopeId,
    stats: NativeResourceStats,
    unique: JitUniqueRuntime,
    structural: JitStructuralRuntime,
    lists: lkjscript_core::SegmentedListArena<lkjscript_core::Value>,
    list_owners: Vec<lkjscript_core::Value>,
    list_allocations: u64,
    max_list_allocations: u64,
    max_runtime_bytes: u64,
}

impl JitIslandServices {
    pub(crate) fn new(scope: ScopeId, config: &ExecutionConfig) -> Result<Self, EngineError> {
        let max_handles = config.max_handles.min(u32::MAX as usize);
        let limits = ResourceTableLimits::new(
            max_handles.max(1),
            max_handles,
            0,
            max_handles,
            0,
            NonZeroU64::new(u64::from(u32::MAX)).ok_or_else(config_error)?,
        )
        .map_err(|error| config_error().with_detail(error.to_string()))?;
        Ok(Self {
            table: ResourceTable::new(scope, limits),
            stdin: None,
            provider: ProviderId::for_capability(CapabilityKind::Stdio),
            scope,
            stats: NativeResourceStats::default(),
            unique: JitUniqueRuntime::new(config)?,
            structural: JitStructuralRuntime::new(config)?,
            lists: lkjscript_core::SegmentedListArena::new(
                lkjscript_core::SegmentedListArenaLimits::default(),
            )
            .map_err(|error| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    None,
                    format!("native island list configuration: {error:?}"),
                )
            })?,
            list_owners: Vec::new(),
            list_allocations: 0,
            max_list_allocations: config.max_allocations,
            max_runtime_bytes: u64::try_from(config.max_heap_bytes).unwrap_or(u64::MAX),
        })
    }

    pub(crate) fn export_unique(
        &mut self,
        owner: NativeUnique,
    ) -> Result<Vec<u8>, NativeServiceError> {
        self.unique.export_owner(owner)
    }

    pub(crate) fn export_structural(
        &mut self,
        owner: lkjscript_native::NativeStructuralOwner,
    ) -> Result<lkjscript_core::SemanticValue, NativeServiceError> {
        self.structural.export(owner)
    }

    pub(crate) fn finish(
        mut self,
    ) -> (
        NativeResourceStats,
        NativeUniqueStats,
        NativeStructuralStats,
        Option<ResourceLimitKind>,
        Option<String>,
        bool,
        lkjscript_core::SegmentedListArena<lkjscript_core::Value>,
    ) {
        if let Some(key) = self.stdin.take() {
            if self
                .table
                .remove_borrowed(key, ResourceKind::InputStream, self.provider, self.scope)
                .is_ok()
            {
                self.stats.borrowed_removals += 1;
            } else {
                self.stats.teardown_failures += 1;
            }
        }
        let table_stats = self.table.stats();
        self.stats.ordinary_obligations = table_stats.ordinary_obligations() as u64;
        self.stats.borrowed_obligations = table_stats.borrowed_open() as u64;
        self.stats.emergency_obligations = self.table.emergency_obligations().count() as u64;
        let unique_resource = self.unique.last_resource();
        let unique = self.unique.finish();
        self.release_list_owners();
        let last_trap = self.structural.take_last_trap();
        let (structural, structural_resource) = self.structural.finish();
        let empty = structural.teardown_failures == 0;
        (
            self.stats,
            unique,
            structural,
            structural_resource.or(unique_resource),
            last_trap,
            empty,
            self.lists,
        )
    }

    fn native_stdin(&mut self) -> Result<NativeResource, NativeServiceError> {
        let key = if let Some(key) = &self.stdin {
            self.stats.borrowed_reuses += 1;
            key.clone()
        } else {
            self.stats.reservations += 1;
            let reservation = self
                .table
                .reserve_borrowed(ResourceKind::InputStream, self.provider)
                .map_err(|_| NativeServiceError::ResourceLimitExceeded)?;
            let key = reservation.commit(BorrowedStandardInput);
            self.stats.borrowed_installs += 1;
            self.stdin = Some(key.clone());
            key
        };
        let parts = key.token_parts();
        let slot =
            u64::try_from(parts.slot()).map_err(|_| NativeServiceError::ResourceLimitExceeded)?;
        let word = parts
            .generation()
            .get()
            .checked_shl(32)
            .and_then(|generation| generation.checked_add(slot))
            .ok_or(NativeServiceError::ResourceLimitExceeded)?;
        Ok(NativeResource::new(ResourceKind::InputStream, word))
    }
}

fn config_error() -> EngineError {
    EngineError::new(
        FailureCode::InvocationFailure,
        None,
        "native resource-table configuration is invalid",
    )
}

trait EngineErrorDetail {
    fn with_detail(self, detail: String) -> Self;
}

impl EngineErrorDetail for EngineError {
    fn with_detail(self, detail: String) -> Self {
        EngineError::new(self.code(), self.function(), detail)
    }
}
