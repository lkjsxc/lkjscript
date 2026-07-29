const MAX_NAME_BYTES: usize = 64;
const MAX_PATH_BYTES: usize = 4_096;
const MAX_ARGUMENTS: usize = 256;
const MAX_ARGUMENT_BYTES: usize = 4_096;
const MAX_ARGUMENT_TOTAL: usize = 16 * 1024;

fn encode_install(
    bytes: &mut Vec<u8>,
    request: &ApplicationInstallRequest,
) -> Result<(), ControlError> {
    text(bytes, &request.name, MAX_NAME_BYTES)?;
    bytes.extend_from_slice(&request.package);
    text(bytes, &request.package_root, MAX_PATH_BYTES)?;
    text(bytes, &request.entry, MAX_PATH_BYTES)?;
    if request.capabilities.len() > lkjscript_core::CapabilityKind::ALL.len()
        || !request.capabilities.windows(2).all(|pair| pair[0] < pair[1])
    {
        return Err(ControlError::Malformed("capability grants"));
    }
    bytes.push(
        u8::try_from(request.capabilities.len()).map_err(|_| ControlError::Oversized)?,
    );
    for capability in &request.capabilities {
        bytes.push(*capability as u8);
    }
    if !(1..=64).contains(&request.max_concurrent_invocations)
        || request.max_total_invocations == 0
    {
        return Err(ControlError::Malformed("application quota"));
    }
    bytes.extend_from_slice(&request.max_concurrent_invocations.to_le_bytes());
    bytes.extend_from_slice(&request.max_total_invocations.to_le_bytes());
    Ok(())
}

fn decode_install(input: &mut Input<'_>) -> Result<ApplicationInstallRequest, ControlError> {
    let name = input.text(MAX_NAME_BYTES)?;
    let package = input.array::<32>()?;
    if package == [0; 32] {
        return Err(ControlError::Malformed("package identity"));
    }
    let package_root = input.text(MAX_PATH_BYTES)?;
    let entry = input.text(MAX_PATH_BYTES)?;
    let count = usize::from(input.u8()?);
    if count > lkjscript_core::CapabilityKind::ALL.len() {
        return Err(ControlError::Malformed("capability count"));
    }
    let mut capabilities = Vec::with_capacity(count);
    for _ in 0..count {
        capabilities.push(
            lkjscript_core::CapabilityKind::from_tag(input.u8()?)
                .ok_or(ControlError::Malformed("capability tag"))?,
        );
    }
    if !capabilities.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(ControlError::Malformed("capability grants"));
    }
    let max_concurrent_invocations = input.u16()?;
    let max_total_invocations = input.u64()?;
    if !(1..=64).contains(&max_concurrent_invocations) || max_total_invocations == 0 {
        return Err(ControlError::Malformed("application quota"));
    }
    Ok(ApplicationInstallRequest {
        name,
        package,
        package_root,
        entry,
        capabilities,
        max_concurrent_invocations,
        max_total_invocations,
    })
}

fn encode_arguments(bytes: &mut Vec<u8>, arguments: &[String]) -> Result<(), ControlError> {
    if arguments.len() > MAX_ARGUMENTS {
        return Err(ControlError::Oversized);
    }
    put_u16(bytes, arguments.len())?;
    let mut total = 0_usize;
    for argument in arguments {
        total = total
            .checked_add(argument.len())
            .ok_or(ControlError::Oversized)?;
        if total > MAX_ARGUMENT_TOTAL {
            return Err(ControlError::Oversized);
        }
        text(bytes, argument, MAX_ARGUMENT_BYTES)?;
    }
    Ok(())
}

fn decode_arguments(input: &mut Input<'_>) -> Result<Vec<String>, ControlError> {
    let count = usize::from(input.u16()?);
    if count > MAX_ARGUMENTS {
        return Err(ControlError::Oversized);
    }
    let mut total = 0_usize;
    let mut arguments = Vec::with_capacity(count);
    for _ in 0..count {
        let argument = input.text(MAX_ARGUMENT_BYTES)?;
        total += argument.len();
        if total > MAX_ARGUMENT_TOTAL {
            return Err(ControlError::Oversized);
        }
        arguments.push(argument);
    }
    Ok(arguments)
}

fn nonzero(bytes: &mut Vec<u8>, value: u64) -> Result<(), ControlError> {
    if value == 0 {
        return Err(ControlError::InvalidIdentity);
    }
    bytes.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn text(bytes: &mut Vec<u8>, value: &str, maximum: usize) -> Result<(), ControlError> {
    if value.is_empty() || value.len() > maximum {
        return Err(ControlError::Malformed("text field"));
    }
    put_u16(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn put_u16(bytes: &mut Vec<u8>, value: usize) -> Result<(), ControlError> {
    bytes.extend_from_slice(
        &u16::try_from(value)
            .map_err(|_| ControlError::Oversized)?
            .to_le_bytes(),
    );
    Ok(())
}

struct Input<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

include!("request_input.rs");
