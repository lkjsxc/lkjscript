use super::*;

#[test]
fn call_frame_stack_policy_rejects_before_wide_local_allocation() {
    let mut chunk = Chunk::new();
    let mut function = chunk.main.clone();
    function.name = "wide-frame".into();
    function.locals = 300;
    function.emit(Op::Unit);
    function.emit(Op::Return);
    chunk.protos.push(function);
    let name = chunk
        .add_const(Constant::Proto(0))
        .expect("add prototype constant");
    chunk.main.emit_op_u64(Op::LoadConst, name.0);
    chunk.main.emit_op_u64(Op::MakeClosure, 0);
    chunk.main.emit_op_u64(Op::Call, 0);
    chunk.main.emit(Op::Return);
    let chunk = validate(chunk);
    let mut policy =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    policy
        .limited_policy_mut()
        .expect("limited test policy")
        .max_stack_values = 10;
    assert_eq!(
        Vm::new(&chunk, NullJit, crate::ExecutionInputs::default(), policy,).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues)
    );
}

#[test]
fn configured_stack_frame_heap_allocation_and_output_limits_stop_execution() {
    let returned = validated(&[Op::Unit, Op::Return]);

    let mut stack =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    stack
        .limited_policy_mut()
        .expect("limited test policy")
        .max_stack_values = 0;
    assert_eq!(
        Vm::new(&returned, NullJit, crate::ExecutionInputs::default(), stack).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues)
    );

    let mut wide_frame = Chunk::new();
    wide_frame.main.locals = 300;
    wide_frame.main.emit(Op::Unit);
    wide_frame.main.emit(Op::Return);
    let wide_frame = validate(wide_frame);
    let mut narrow_policy =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    narrow_policy
        .limited_policy_mut()
        .expect("limited test policy")
        .max_stack_values = 1;
    assert_eq!(
        Vm::new(
            &wide_frame,
            NullJit,
            crate::ExecutionInputs::default(),
            narrow_policy,
        )
        .run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues)
    );

    let mut frames =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    frames
        .limited_policy_mut()
        .expect("limited test policy")
        .max_frames = 0;
    assert_eq!(
        Vm::new(
            &returned,
            NullJit,
            crate::ExecutionInputs::default(),
            frames
        )
        .run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::FrameDepth)
    );

    let mut string = Chunk::new();
    let text = string
        .add_const(Constant::Str("x".into()))
        .expect("add text constant");
    string.main.emit_op_u64(Op::LoadConst, text.0);
    string.main.emit(Op::Return);
    let string = validate(string);

    let mut no_heap =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    no_heap
        .limited_policy_mut()
        .expect("limited test policy")
        .max_heap_bytes = 0;
    no_heap
        .limited_policy_mut()
        .expect("limited test policy")
        .max_allocations = 0;
    assert!(matches!(
        Vm::new(&string, NullJit, crate::ExecutionInputs::default(), no_heap).run(),
        ExecutionOutcome::Returned(value) if value.as_str() == Some("x")
    ));

    let mut aggregate = Chunk::new();
    aggregate.main.emit(Op::Unit);
    aggregate.main.emit(Op::EmptyList);
    aggregate.main.emit(Op::Cons);
    aggregate.main.emit(Op::Return);
    let aggregate = validate(aggregate);

    let mut heap = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    heap.limited_policy_mut()
        .expect("limited test policy")
        .max_heap_bytes = 0;
    assert_eq!(
        Vm::new(&aggregate, NullJit, crate::ExecutionInputs::default(), heap).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
    );

    let mut allocations =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    allocations
        .limited_policy_mut()
        .expect("limited test policy")
        .max_allocations = 0;
    assert_eq!(
        Vm::new(
            &aggregate,
            NullJit,
            crate::ExecutionInputs::default(),
            allocations
        )
        .run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
    );

    let mut output_chunk = Chunk::new();
    output_chunk.required_capabilities = vec![lkjscript_core::CapabilityKind::Stdio];
    output_chunk.main.arity = 1;
    output_chunk.main.locals = 1;
    let text = output_chunk
        .add_const(Constant::Str("x".into()))
        .expect("add text constant");
    output_chunk.main.emit_op_u64(Op::LoadLocal, 0);
    output_chunk.main.emit_op_u64(Op::LoadConst, text.0);
    output_chunk.main.emit(Op::WriteStr);
    output_chunk.main.emit(Op::Return);
    let output_chunk = validate(output_chunk);
    let mut output =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    output
        .limited_policy_mut()
        .expect("limited test policy")
        .max_output_bytes = 0;
    assert_eq!(
        Vm::new(
            &output_chunk,
            NullJit,
            capability_inputs(lkjscript_core::CapabilityKind::Stdio),
            output
        )
        .run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::OutputBytes)
    );

    let mut hard_deadline =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    hard_deadline
        .limited_policy_mut()
        .expect("limited test policy")
        .require_hard_deadline = true;
    assert!(matches!(
        Vm::new(
            &output_chunk,
            NullJit,
            capability_inputs(lkjscript_core::CapabilityKind::Stdio),
            hard_deadline,
        )
        .run(),
        ExecutionOutcome::HostFailure(error)
            if error.as_str().contains("hard wall deadline is unsupported")
    ));
}
#[test]
fn configured_handle_and_wall_limits_are_structured() {
    let mut socket = Chunk::new();
    socket.required_capabilities = vec![lkjscript_core::CapabilityKind::Network];
    socket.main.arity = 1;
    socket.main.locals = 1;
    socket.main.emit_op_u64(Op::LoadLocal, 0);
    socket.main.emit(Op::SysSocket);
    let wait_after_success = socket.main.code.len();
    socket.main.emit_op_u64(
        Op::Jump,
        u64::try_from(wait_after_success).expect("test offset fits u64"),
    );
    let socket = validate(socket);
    let mut handles =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    handles
        .limited_policy_mut()
        .expect("limited test policy")
        .max_handles = 0;
    assert_eq!(
        Vm::new(
            &socket,
            NullJit,
            capability_inputs(lkjscript_core::CapabilityKind::Network),
            handles,
        )
        .run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Handles)
    );

    let mut loop_chunk = Chunk::new();
    loop_chunk.main.emit_op_u64(Op::Jump, 0);
    let loop_chunk = validate(loop_chunk);
    let mut deadline =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    deadline
        .limited_policy_mut()
        .expect("limited test policy")
        .wall_time = Some(Duration::ZERO);
    assert_eq!(
        Vm::new(
            &loop_chunk,
            NullJit,
            crate::ExecutionInputs::default(),
            deadline
        )
        .run(),
        ExecutionOutcome::DeadlineExceeded
    );

    let mut wait = Chunk::new();
    wait.required_capabilities = vec![lkjscript_core::CapabilityKind::Clock];
    wait.main.arity = 1;
    wait.main.locals = 1;
    let duration = wait
        .add_const(Constant::I64(50))
        .expect("add duration constant");
    wait.main.emit_op_u64(Op::LoadLocal, 0);
    wait.main.emit_op_u64(Op::LoadConst, duration.0);
    wait.main.emit(Op::SysWaitMs);
    wait.main.emit(Op::Return);
    let wait = validate(wait);
    let mut deadline =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    deadline
        .limited_policy_mut()
        .expect("limited test policy")
        .wall_time = Some(Duration::from_millis(1));
    assert_eq!(
        Vm::new(
            &wait,
            NullJit,
            capability_inputs(lkjscript_core::CapabilityKind::Clock),
            deadline,
        )
        .run(),
        ExecutionOutcome::DeadlineExceeded
    );
}

fn capability_inputs(kind: lkjscript_core::CapabilityKind) -> crate::ExecutionInputs {
    crate::ExecutionInputs {
        arguments: Vec::new(),
        capabilities: vec![kind],
        host: lkjscript_host::HostEnvironment::portable(),
    }
}
