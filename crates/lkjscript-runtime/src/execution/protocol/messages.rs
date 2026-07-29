fn encode_bootstrap(output: &mut Writer, value: &ProcessBootstrap) -> io::Result<()> {
    output.u64(nonzero(value.platform_revision, "platform revision")?)?;
    output.extend(&value.contract)?;
    output.u64(nonzero(value.coordinator, "coordinator")?)?;
    output.u64(nonzero(value.application, "application")?)?;
    output.u64(nonzero(value.incarnation, "incarnation")?)?;
    output.extend(&value.package)?;
    output.text(&value.entry, MAX_ENTRY_BYTES)?;
    encode_capabilities(output, &value.capabilities)?;
    encode_config(output, &value.execution)
}

fn decode_bootstrap(input: &mut Reader<'_>) -> io::Result<ProcessBootstrap> {
    let platform_revision = nonzero(input.u64()?, "platform revision")?;
    let contract = input
        .take(32)?
        .try_into()
        .map_err(|_| invalid("contract digest length"))?;
    let coordinator = nonzero(input.u64()?, "coordinator")?;
    let application = nonzero(input.u64()?, "application")?;
    let incarnation = nonzero(input.u64()?, "incarnation")?;
    let package = input
        .take(32)?
        .try_into()
        .map_err(|_| invalid("package digest length"))?;
    if contract == [0; 32] || package == [0; 32] {
        return Err(invalid("process digest must be nonzero"));
    }
    Ok(ProcessBootstrap {
        platform_revision,
        contract,
        coordinator,
        application,
        incarnation,
        package,
        entry: input.text(MAX_ENTRY_BYTES)?,
        capabilities: decode_capabilities(input)?,
        execution: decode_config(input)?,
    })
}

fn encode_capabilities(output: &mut Writer, capabilities: &[CapabilityKind]) -> io::Result<()> {
    if capabilities.len() > CapabilityKind::ALL.len()
        || !capabilities.windows(2).all(|pair| pair[0] < pair[1])
    {
        return Err(invalid("process capabilities must be sorted and unique"));
    }
    output.u8(
        u8::try_from(capabilities.len()).map_err(|_| invalid("capability count exceeds u8"))?,
    )?;
    for capability in capabilities {
        output.u8(*capability as u8)?;
    }
    Ok(())
}

fn decode_capabilities(input: &mut Reader<'_>) -> io::Result<Vec<CapabilityKind>> {
    let count = usize::from(input.u8()?);
    if count > CapabilityKind::ALL.len() {
        return Err(invalid("capability count exceeds bound"));
    }
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(
            CapabilityKind::from_tag(input.u8()?)
                .ok_or_else(|| invalid("unknown process capability"))?,
        );
    }
    if !values.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(invalid("process capabilities must be sorted and unique"));
    }
    Ok(values)
}

fn encode_arguments(output: &mut Writer, arguments: &[String]) -> io::Result<()> {
    if arguments.len() > MAX_ARGUMENTS {
        return Err(invalid("process argument count exceeds bound"));
    }
    let aggregate = arguments.iter().try_fold(0_usize, |sum, value| {
        sum.checked_add(value.len())
    });
    if aggregate.is_none_or(|value| value > MAX_AGGREGATE_ARGUMENT_BYTES) {
        return Err(invalid("process argument bytes exceed bound"));
    }
    output.u32(u32::try_from(arguments.len()).map_err(|_| invalid("argument count"))?)?;
    for argument in arguments {
        output.text(argument, MAX_ARGUMENT_BYTES)?;
    }
    Ok(())
}

fn decode_arguments(input: &mut Reader<'_>) -> io::Result<Vec<String>> {
    let count = usize::try_from(input.u32()?).map_err(|_| invalid("argument count"))?;
    if count > MAX_ARGUMENTS {
        return Err(invalid("process argument count exceeds bound"));
    }
    let mut aggregate = 0_usize;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let value = input.text(MAX_ARGUMENT_BYTES)?;
        aggregate = aggregate
            .checked_add(value.len())
            .ok_or_else(|| invalid("process argument bytes overflow"))?;
        if aggregate > MAX_AGGREGATE_ARGUMENT_BYTES {
            return Err(invalid("process argument bytes exceed bound"));
        }
        values.push(value);
    }
    Ok(values)
}

include!("responses.rs");
