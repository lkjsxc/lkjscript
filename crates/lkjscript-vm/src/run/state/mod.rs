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
        let (structural, structural_initialization_error) =
            match structural_ops::StructuralInvocation::new(config.max_allocations) {
                Ok(runtime) => (Some(runtime), None),
                Err(error) => (None, Some(error)),
            };
        Self {
            chunk,
            globals: vec![Value::INVALID; chunk.global_names().len()],
            stack: Vec::new(),
            frames: Vec::new(),
            arena: Arena::new(GcConfig {
                max_allocations: config.max_allocations,
                max_heap_bytes: config.max_heap_bytes,
                ..GcConfig::default()
            }),
            jit,
            exit_code: None,
            inputs,
            resources: ResourceTable::new(config.max_handles, config.cleanup_failure_limits),
            unique: unique::UniqueRuntime::new(&config),
            structural,
            structural_initialization_error,
            fuel_remaining: config.instruction_fuel,
            output_bytes: 0,
            allocation_error: None,
            cleanup_failures: CleanupFailures::new(config.cleanup_failure_limits),
            logical_aggregate_constructions: 0,
            started: Instant::now(),
            config,
        }
    }
}
