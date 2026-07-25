use super::*;

#[test]
fn configured_stack_frame_heap_allocation_and_output_limits_stop_execution() {
    let returned = validated(&[Op::Unit, Op::Return]);

    let mut stack = ExecutionConfig::default();
    stack.max_stack_values = 0;
    assert_eq!(
        Vm::new(&returned, NullJit, Vec::new(), stack).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues)
    );

    let mut frames = ExecutionConfig::default();
    frames.max_frames = 0;
    assert_eq!(
        Vm::new(&returned, NullJit, Vec::new(), frames).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::FrameDepth)
    );

    let mut string = Chunk::new();
    let text = string.add_const(Constant::Str("x".into()));
    string.main.emit_op_u16(Op::LoadConst, text.0);
    string.main.emit(Op::Return);
    let string = validate(string);

    let mut heap = ExecutionConfig::default();
    heap.max_heap_bytes = 0;
    assert_eq!(
        Vm::new(&string, NullJit, Vec::new(), heap).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
    );

    let mut allocations = ExecutionConfig::default();
    allocations.max_allocations = 0;
    assert_eq!(
        Vm::new(&string, NullJit, Vec::new(), allocations).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
    );

    let mut output_chunk = Chunk::new();
    let text = output_chunk.add_const(Constant::Str("x".into()));
    output_chunk.main.emit_op_u16(Op::LoadConst, text.0);
    output_chunk.main.emit(Op::WriteStr);
    output_chunk.main.emit(Op::Return);
    let output_chunk = validate(output_chunk);
    let mut output = ExecutionConfig::default();
    output.max_output_bytes = 0;
    assert_eq!(
        Vm::new(&output_chunk, NullJit, Vec::new(), output).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::OutputBytes)
    );

    let mut hard_deadline = ExecutionConfig::default();
    hard_deadline.require_hard_deadline = true;
    assert!(matches!(
        Vm::new(&output_chunk, NullJit, Vec::new(), hard_deadline).run(),
        ExecutionOutcome::HostFailure(error)
            if error.as_str().contains("hard wall deadline is unsupported")
    ));
}
#[test]
fn configured_handle_and_wall_limits_are_structured() {
    let socket = validated(&[Op::SysSocket, Op::Return]);
    let mut handles = ExecutionConfig::default();
    handles.max_handles = 0;
    assert_eq!(
        Vm::new(&socket, NullJit, Vec::new(), handles).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Handles)
    );

    let mut loop_chunk = Chunk::new();
    loop_chunk.main.emit_op_u16(Op::Jump, 0);
    let loop_chunk = validate(loop_chunk);
    let mut deadline = ExecutionConfig::default();
    deadline.wall_time = Some(Duration::ZERO);
    assert_eq!(
        Vm::new(&loop_chunk, NullJit, Vec::new(), deadline).run(),
        ExecutionOutcome::DeadlineExceeded
    );

    let mut wait = Chunk::new();
    let duration = wait.add_const(Constant::I64(50));
    wait.main.emit_op_u16(Op::LoadConst, duration.0);
    wait.main.emit(Op::SysWaitMs);
    wait.main.emit(Op::Return);
    let wait = validate(wait);
    let mut deadline = ExecutionConfig::default();
    deadline.wall_time = Some(Duration::from_millis(1));
    assert_eq!(
        Vm::new(&wait, NullJit, Vec::new(), deadline).run(),
        ExecutionOutcome::DeadlineExceeded
    );
}
