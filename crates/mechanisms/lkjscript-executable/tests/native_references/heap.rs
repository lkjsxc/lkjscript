use super::*;

struct FailingHeapService {
    failure: HeapFailure,
    calls: usize,
}

impl NativeRuntimeServices for FailingHeapService {
    fn collect_references(&mut self, _roots: &mut [NativeRoot]) -> Result<(), NativeServiceError> {
        Ok(())
    }

    fn heap_operation(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        assert_eq!(arguments.len(), 3);
        assert_eq!(site.arguments().len(), 3);
        self.calls += 1;
        Err(match self.failure {
            HeapFailure::Trap => NativeServiceError::Trap,
            HeapFailure::Resource => NativeServiceError::ResourceLimitExceeded,
            HeapFailure::Host => NativeServiceError::HostFailure,
        })
    }
}

struct MovingHeapService {
    expected_word: u64,
    observed_word: Option<u64>,
}

impl NativeRuntimeServices for MovingHeapService {
    fn collect_references(&mut self, _roots: &mut [NativeRoot]) -> Result<(), NativeServiceError> {
        Ok(())
    }

    fn prepare_heap_operation(
        &mut self,
        _site: &HeapRuntimeSite,
        _arguments: &[NativeValue],
        roots: &mut [NativeRoot],
    ) -> Result<bool, NativeServiceError> {
        for root in roots {
            root.set_opaque_word(self.expected_word);
        }
        Ok(true)
    }

    fn heap_operation(
        &mut self,
        _site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        self.observed_word = arguments.first().and_then(|argument| match argument {
            NativeValue::Reference(reference) => Some(reference.opaque_word()),
            _ => None,
        });
        Ok(NativeValue::I64(7))
    }
}

#[test]
fn heap_dispatch_rematerializes_moved_argument_after_root_writeback(
) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(41),
        Signature::new(vec![buffer], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let descriptor = HeapCallDescriptor::new(
        HeapOperation::BufLen,
        vec![buffer],
        ValueType::I64,
        AllocationClass::None,
        StoreClass::None,
    )?;
    let result = builder.heap_call(entry, descriptor, vec![input])?;
    builder.return_value(entry, result)?;
    plan.define_function(builder.finish())?;
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    let installed = ExecutableInstaller::default().install(image)?;
    let mut service = MovingHeapService {
        expected_word: 99,
        observed_word: None,
    };
    let report = installed.invoke_with_services(
        function,
        &[buf(11)],
        &NativeInvocationConfig::default(),
        &mut service,
    )?;
    assert_eq!(
        report.outcome(),
        InvocationOutcome::Returned(NativeValue::I64(7))
    );
    assert_eq!(service.observed_word, Some(99));
    assert_eq!(report.heap_operation_attempts(), 1);
    assert_eq!(report.heap_operation_successes(), 1);
    Ok(())
}

#[test]
fn generic_heap_dispatch_propagates_service_status_and_unwinds(
) -> Result<(), Box<dyn std::error::Error>> {
    let product = ValueType::Reference(ReferenceType::Product(LayoutIdentity::product(0)));
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(42),
        Signature::new(Vec::new(), product)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let values = [
        builder.i64_const(entry, 1)?,
        builder.i64_const(entry, 2)?,
        builder.i64_const(entry, 3)?,
    ];
    let descriptor = HeapCallDescriptor::new(
        HeapOperation::ProductValue {
            product: 0,
            fields: 3,
        },
        vec![ValueType::I64; 3],
        product,
        AllocationClass::Bounded,
        StoreClass::Initialization,
    )?;
    let result = builder.heap_call(entry, descriptor, values.to_vec())?;
    builder.return_value(entry, result)?;
    plan.define_function(builder.finish())?;
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    let installed = ExecutableInstaller::default().install(image)?;
    for (failure, expected) in [
        (
            HeapFailure::Trap,
            InvocationOutcome::Trapped(TrapCode::Explicit),
        ),
        (
            HeapFailure::Resource,
            InvocationOutcome::ResourceLimitExceeded(NativeResourceLimitKind::RuntimeService),
        ),
        (HeapFailure::Host, InvocationOutcome::HostFailure),
    ] {
        let mut service = FailingHeapService { failure, calls: 0 };
        let report = installed.invoke_with_services(
            function,
            &[],
            &NativeInvocationConfig::default(),
            &mut service,
        )?;
        assert_eq!(report.outcome(), expected);
        assert_eq!(report.active_frame_depth(), 0);
        assert_eq!(report.reserved_native_stack_bytes(), 0);
        assert_eq!(report.heap_operation_attempts(), 1);
        assert_eq!(report.heap_operation_successes(), 0);
        assert_eq!(service.calls, 1);
    }
    Ok(())
}
