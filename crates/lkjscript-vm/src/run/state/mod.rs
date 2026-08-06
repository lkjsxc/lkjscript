mod execution;
mod lifecycle;
mod limits;

use super::*;

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub fn new(
        chunk: &'a ValidatedChunk,
        jit: J,
        inputs: ExecutionInputs,
        config: ExecutionConfig,
    ) -> Self {
        let mut globals = Vec::new();
        let global_initialization_error = globals
            .try_reserve_exact(chunk.global_names().len())
            .map_err(|_| Error::host("VM global table allocation failed"))
            .err();
        if global_initialization_error.is_none() {
            globals.resize(chunk.global_names().len(), Value::INVALID);
        }
        let (structural, structural_initialization_error) =
            match structural_ops::StructuralInvocation::new(config.max_allocations) {
                Ok(runtime) => (Some(runtime), None),
                Err(error) => (None, Some(error)),
            };
        let (lists, list_initialization_error) = match lkjscript_core::SegmentedListArena::new(
            lkjscript_core::SegmentedListArenaLimits::default(),
        ) {
            Ok(lists) => (Some(lists), None),
            Err(error) => (
                None,
                Some(Error::msg(format!(
                    "segmented-list arena initialization failed: {error:?}"
                ))),
            ),
        };
        let region_limit = u32::try_from(config.max_allocations)
            .unwrap_or(u32::MAX)
            .max(1);
        let region_limits = lkjscript_core::RegionProductLimits {
            max_records: std::num::NonZeroU32::new(region_limit)
                .unwrap_or(std::num::NonZeroU32::MIN),
            max_fields: std::num::NonZeroU32::new(region_limit.saturating_mul(16))
                .unwrap_or(std::num::NonZeroU32::MIN),
        };
        let (region_products, region_product_initialization_error) =
            match lkjscript_core::RegionProductArena::new(region_limits) {
                Ok(arena) => (Some(arena), None),
                Err(error) => (
                    None,
                    Some(Error::msg(format!(
                        "region-product arena initialization failed: {error:?}"
                    ))),
                ),
            };
        Self {
            chunk,
            globals,
            stack: Vec::new(),
            frames: Vec::new(),
            lists,
            region_products,
            jit,
            exit_code: None,
            inputs,
            resources: ResourceTable::new(config.max_handles, config.cleanup_failure_limits),
            unique: unique::UniqueRuntime::new(&config),
            structural,
            structural_initialization_error,
            global_initialization_error,
            list_initialization_error,
            region_product_initialization_error,
            fuel_remaining: config.instruction_fuel,
            output_bytes: 0,
            allocation_error: None,
            cleanup_failures: CleanupFailures::new(config.cleanup_failure_limits),
            logical_aggregate_constructions: 0,
            list_allocations: 0,
            region_product_allocations: 0,
            started: Instant::now(),
            config,
        }
    }
}
