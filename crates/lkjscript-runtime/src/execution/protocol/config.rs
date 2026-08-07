fn encode_config(output: &mut Writer, value: &ExecutionPolicy) -> io::Result<()> {
    let Some(value) = value.limited_policy() else {
        return output.u8(0);
    };
    output.u8(1)?;
    output.u64(value.instruction_fuel)?;
    output.usize(value.max_stack_values)?;
    output.usize(value.max_frames)?;
    output.usize(value.max_heap_bytes)?;
    output.u64(value.max_allocations)?;
    output.usize(value.max_handles)?;
    output.usize(value.max_output_bytes)?;
    output.usize(value.cleanup_retention.max_failures())?;
    output.usize(value.cleanup_retention.max_message_bytes())?;
    match value.wall_time {
        Some(duration) => {
            output.u8(1)?;
            output.u64(duration.as_secs())?;
            output.u32(duration.subsec_nanos())?;
        }
        None => output.u8(0)?,
    }
    output.u8(u8::from(value.require_hard_deadline))
}

fn decode_config(input: &mut Reader<'_>) -> io::Result<ExecutionPolicy> {
    match input.u8()? {
        0 => Ok(ExecutionPolicy::unrestricted()),
        1 => decode_limited_config(input).map(ExecutionPolicy::limited),
        _ => Err(invalid("unknown process execution policy tag")),
    }
}

fn decode_limited_config(input: &mut Reader<'_>) -> io::Result<LimitedExecutionPolicy> {
    let instruction_fuel = input.u64()?;
    let max_stack_values = input.usize()?;
    let max_frames = input.usize()?;
    let max_heap_bytes = input.usize()?;
    let max_allocations = input.u64()?;
    let max_handles = input.usize()?;
    let max_output_bytes = input.usize()?;
    let cleanup_retention =
        lkjscript_core::CleanupFailureLimits::new(input.usize()?, input.usize()?)
            .ok_or_else(|| invalid("process cleanup retention exceeds bounds"))?;
    let wall_time = match input.u8()? {
        0 => None,
        1 => {
            let seconds = input.u64()?;
            let nanos = input.u32()?;
            if nanos >= 1_000_000_000 {
                return Err(invalid("process wall duration nanos exceed bound"));
            }
            Some(std::time::Duration::new(seconds, nanos))
        }
        _ => return Err(invalid("unknown process wall duration tag")),
    };
    let require_hard_deadline = match input.u8()? {
        0 => false,
        1 => true,
        _ => return Err(invalid("unknown process deadline boolean")),
    };
    Ok(LimitedExecutionPolicy {
        instruction_fuel,
        max_stack_values,
        max_frames,
        max_heap_bytes,
        max_allocations,
        max_handles,
        max_output_bytes,
        cleanup_retention,
        wall_time,
        require_hard_deadline,
    })
}
