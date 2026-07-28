use crate::*;

impl JitSession {
    pub(super) fn invoke_collector_free(
        &mut self,
        function: FunctionId,
        object_index: usize,
        native: lkjscript_native::FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        execution: &ExecutionConfig,
    ) -> Result<InvocationReport, EngineError> {
        let scope = lkjscript_core::ScopeId::new(self.next_resource_scope).ok_or_else(|| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                "native resource scope exhausted",
            )
        })?;
        self.next_resource_scope = self.next_resource_scope.checked_add(1).ok_or_else(|| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                "native resource scope exhausted",
            )
        })?;
        let mut services = JitIslandServices::new(scope, execution)?;
        let report = self.objects[object_index]
            .installed
            .invoke_island_with_services(native, arguments, config, &mut services);
        let export = report
            .as_ref()
            .ok()
            .and_then(|report| match report.outcome() {
                InvocationOutcome::Returned(NativeValue::Unique(owner)) => {
                    Some(services.export_unique(owner))
                }
                _ => None,
            });
        let (resources, unique, last_resource) = services.finish();
        self.native_resources.add(resources);
        self.native_unique.add(unique);
        self.last_runtime_resource = last_resource;
        match export {
            Some(Ok(bytes)) => self.returned_unique = Some(bytes),
            Some(Err(_)) => {
                return Err(EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "native byte-vector return transfer failed",
                ));
            }
            None => {}
        }
        report.map_err(|error| invocation_error(function, error))
    }
}
