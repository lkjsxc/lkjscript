mod execution;
mod lifecycle;
mod limits;

use super::*;

impl<'a> Vm<'a> {
    pub fn new(
        chunk: &'a ValidatedChunk,
        inputs: ExecutionInputs,
        config: ExecutionPolicy,
    ) -> Self {
        Self::new_started(chunk, inputs, config, Instant::now())
    }

    pub(crate) fn new_started(
        chunk: &'a ValidatedChunk,
        inputs: ExecutionInputs,
        config: ExecutionPolicy,
        started: Instant,
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
            match structural_ops::StructuralInvocation::new() {
                Ok(runtime) => (Some(runtime), None),
                Err(error) => (None, Some(error)),
            };
        let (lists, list_initialization_error) = match lkjscript_core::SegmentedListArena::new() {
            Ok(lists) => (Some(lists), None),
            Err(error) => (
                None,
                Some(Error::msg(format!(
                    "segmented-list arena initialization failed: {error:?}"
                ))),
            ),
        };
        let (region_products, region_product_initialization_error) =
            match lkjscript_core::RegionProductArena::new() {
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
            exit_code: None,
            inputs,
            resources: ResourceTable::new(config.max_handles(), config.cleanup_retention()),
            unique: unique::UniqueRuntime::new(&config),
            structural,
            structural_initialization_error,
            global_initialization_error,
            list_initialization_error,
            region_product_initialization_error,
            fuel_remaining: config.instruction_fuel(),
            output_bytes: 0,
            allocation_error: None,
            cleanup_failures: CleanupFailures::with_retention(config.cleanup_retention()),
            list_allocations: 0,
            region_product_allocations: 0,
            started,
            config,
        }
    }
}
