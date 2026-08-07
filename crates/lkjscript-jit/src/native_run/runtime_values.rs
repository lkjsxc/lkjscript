use crate::attempt::BaselineRegionAttempt;
use crate::*;

impl NativeRun {
    pub(crate) fn invoke_baseline_region_attempt(
        &mut self,
        function: FunctionId,
        native: lkjscript_native::FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        execution: &ExecutionPolicy,
    ) -> BaselineRegionAttempt {
        let preparation_started = Instant::now();
        if let Err(error) = self.initialize_region_arenas(function, execution.max_allocations()) {
            return BaselineRegionAttempt::PreparationFailure {
                error,
                preparation: preparation_started.elapsed(),
            };
        }
        let Some(lists) = self.lists.as_mut() else {
            return BaselineRegionAttempt::PreparationFailure {
                error: missing_arena(function, "list"),
                preparation: preparation_started.elapsed(),
            };
        };
        let Some(products) = self.region_products.as_mut() else {
            return BaselineRegionAttempt::PreparationFailure {
                error: missing_arena(function, "region-product"),
                preparation: preparation_started.elapsed(),
            };
        };
        let mut services = JitValueServices::new(
            lists,
            products,
            JitValueLimits {
                allocations: execution.max_allocations(),
                runtime_bytes: execution
                    .max_heap_bytes()
                    .and_then(|bytes| u64::try_from(bytes).ok()),
            },
        );
        let Some(object) = self.object.as_ref() else {
            return BaselineRegionAttempt::PreparationFailure {
                error: missing_object(function),
                preparation: preparation_started.elapsed(),
            };
        };
        let prepared =
            object
                .installed
                .prepare_region_invocation(native, arguments, config, &mut services);
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                self.last_runtime_trap = services.last_trap.take();
                self.last_runtime_resource = services.last_resource;
                return BaselineRegionAttempt::Declined {
                    error,
                    preparation: preparation_started.elapsed(),
                };
            }
        };
        let preparation = preparation_started.elapsed();
        let native_started = Instant::now();
        let result = prepared.enter();
        let native_execution = native_started.elapsed();
        self.last_runtime_trap = services.last_trap.take();
        self.last_runtime_resource = services.last_resource;
        BaselineRegionAttempt::Entered {
            result: Box::new(result),
            preparation,
            native_execution,
        }
    }

    pub(crate) fn initialize_region_arenas(
        &mut self,
        function: FunctionId,
        _max_allocations: Option<u64>,
    ) -> Result<(), EngineError> {
        self.lists = Some(
            lkjscript_core::SegmentedListArena::new()
                .map_err(|error| arena_error(function, "segmented-list", error))?,
        );
        self.region_products = Some(
            lkjscript_core::RegionProductArena::new()
                .map_err(|error| arena_error(function, "region-product", error))?,
        );
        Ok(())
    }
}

fn missing_object(function: FunctionId) -> EngineError {
    EngineError::new(
        FailureCode::InvocationFailure,
        Some(function),
        "native run lost its installed group",
    )
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
