use crate::*;

impl JitSession {
    pub(super) fn invoke_invocation_region(
        &mut self,
        function: FunctionId,
        object_index: usize,
        native: lkjscript_native::FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        execution: &ExecutionConfig,
    ) -> Result<InvocationReport, EngineError> {
        self.invoke_value_services(function, object_index, native, arguments, config, execution)
    }

    fn invoke_value_services(
        &mut self,
        function: FunctionId,
        object_index: usize,
        native: lkjscript_native::FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        execution: &ExecutionConfig,
    ) -> Result<InvocationReport, EngineError> {
        self.initialize_region_arenas(function, execution.max_allocations)?;
        let lists = self
            .lists
            .as_mut()
            .ok_or_else(|| missing_arena(function, "list"))?;
        let products = self
            .region_products
            .as_mut()
            .ok_or_else(|| missing_arena(function, "region-product"))?;
        let mut services = JitValueServices::new(
            lists,
            products,
            JitValueLimits {
                logical_aggregates: execution.max_logical_aggregate_constructions,
                allocations: execution.max_allocations,
                runtime_bytes: u64::try_from(execution.max_heap_bytes).unwrap_or(u64::MAX),
            },
        );
        let report = self.objects[object_index].installed.invoke_with_services(
            native,
            arguments,
            config,
            &mut services,
        );
        self.last_runtime_trap = services.last_trap.take();
        self.last_runtime_resource = services.last_resource;
        report.map_err(|error| invocation_error(function, error))
    }

    fn initialize_region_arenas(
        &mut self,
        function: FunctionId,
        max_allocations: u64,
    ) -> Result<(), EngineError> {
        self.lists = Some(
            lkjscript_core::SegmentedListArena::new(
                lkjscript_core::SegmentedListArenaLimits::default(),
            )
            .map_err(|error| arena_error(function, "segmented-list", error))?,
        );
        let records = u32::try_from(max_allocations).unwrap_or(u32::MAX).max(1);
        self.region_products = Some(
            lkjscript_core::RegionProductArena::new(lkjscript_core::RegionProductLimits {
                max_records: std::num::NonZeroU32::new(records)
                    .unwrap_or(std::num::NonZeroU32::MIN),
                max_fields: std::num::NonZeroU32::new(records.saturating_mul(16))
                    .unwrap_or(std::num::NonZeroU32::MIN),
            })
            .map_err(|error| arena_error(function, "region-product", error))?,
        );
        Ok(())
    }
}

fn missing_arena(function: FunctionId, name: &str) -> EngineError {
    EngineError::new(
        FailureCode::InvocationFailure,
        Some(function),
        format!("{name} arena disappeared"),
    )
}

fn arena_error(function: FunctionId, name: &str, error: impl std::fmt::Debug) -> EngineError {
    EngineError::new(
        FailureCode::InvocationFailure,
        Some(function),
        format!("{name} arena initialization failed: {error:?}"),
    )
}
