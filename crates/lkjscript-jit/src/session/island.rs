use crate::*;

impl JitSession {
    pub(crate) fn take_returned_unique(
        &mut self,
        function: FunctionId,
        bytes: bool,
    ) -> Result<OwnedValue, EngineError> {
        let payload = self.returned_unique.take().ok_or_else(|| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                "native unique return has no transferred backing",
            )
        })?;
        let result = if bytes {
            OwnedValue::from_unique_bytes(payload)
        } else {
            OwnedValue::from_unique_byte_vector(payload)
        };
        result.map_err(|error| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                error.to_string(),
            )
        })
    }

    pub fn take_returned_structural(
        &mut self,
        function: FunctionId,
    ) -> Result<SemanticValue, EngineError> {
        self.returned_structural.take().ok_or_else(|| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                "native structural return has no exported semantic tree",
            )
        })
    }

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
        let unique_export = report
            .as_ref()
            .ok()
            .and_then(|report| match report.outcome() {
                InvocationOutcome::Returned(NativeValue::Unique(owner)) => {
                    Some(services.export_unique(owner))
                }
                InvocationOutcome::Returned(NativeValue::StaticBytes(identity)) => Some(
                    self.objects[object_index]
                        .installed
                        .resolve_static_bytes(identity)
                        .map(<[u8]>::to_vec)
                        .ok_or(NativeServiceError::Trap),
                ),
                _ => None,
            });
        let structural_export = report
            .as_ref()
            .ok()
            .and_then(|report| match report.outcome() {
                InvocationOutcome::Returned(NativeValue::StructuralOwner(owner)) => {
                    Some(services.export_structural(owner))
                }
                _ => None,
            });
        let (resources, unique, structural, last_resource, last_trap, empty) = services.finish();
        self.native_resources.add(resources);
        self.native_unique.add(unique);
        self.native_structural.add(structural);
        self.last_runtime_resource = last_resource;
        self.last_runtime_trap = last_trap;
        if !empty {
            return Err(EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                format!(
                    concat!(
                        "native structural runtime completed with live obligations: ",
                        "roots={}, loans={}, views={}, destinations={}, published={}, dropped={}",
                    ),
                    structural.live_roots,
                    structural.live_loans,
                    structural.live_views,
                    structural.live_destinations,
                    structural.roots_published,
                    structural.roots_dropped,
                ),
            ));
        }
        match unique_export {
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
        match structural_export {
            Some(Ok(value)) => self.returned_structural = Some(value),
            Some(Err(_)) => {
                return Err(EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "native structural return export failed",
                ));
            }
            None => {}
        }
        report.map_err(|error| invocation_error(function, error))
    }
}
