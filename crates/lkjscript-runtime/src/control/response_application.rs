fn encode_application(
    bytes: &mut Vec<u8>,
    application: &ControlledApplication,
) -> Result<(), ControlError> {
    put_nonzero(bytes, application.application)?;
    put_text(bytes, &application.name, MAX_NAME_BYTES)?;
    bytes.push(u8::from(application.desired_running));
    bytes.push(match application.state {
        ControlledApplicationState::Installed => 0,
        ControlledApplicationState::Running => 1,
        ControlledApplicationState::Stopped => 2,
        ControlledApplicationState::Failed => 3,
    });
    match application.incarnation {
        Some(incarnation) => {
            bytes.push(1);
            put_nonzero(bytes, incarnation)?;
        }
        None => bytes.push(0),
    }
    match application.process {
        Some(process) if process != 0 => {
            bytes.push(1);
            bytes.extend_from_slice(&process.to_le_bytes());
        }
        Some(_) => return Err(ControlError::InvalidIdentity),
        None => bytes.push(0),
    }
    bytes.push(u8::from(application.database_attached));
    Ok(())
}

fn decode_application(input: &mut ResponseInput<'_>) -> Result<ControlledApplication, ControlError> {
    let application = input.nonzero()?;
    let name = input.text(MAX_NAME_BYTES)?;
    let desired_running = input.boolean()?;
    let state = match input.u8()? {
        0 => ControlledApplicationState::Installed,
        1 => ControlledApplicationState::Running,
        2 => ControlledApplicationState::Stopped,
        3 => ControlledApplicationState::Failed,
        _ => return Err(ControlError::Malformed("application state")),
    };
    let incarnation = input.optional_nonzero()?;
    let process = match input.u8()? {
        0 => None,
        1 => {
            let process = input.u32()?;
            if process == 0 {
                return Err(ControlError::InvalidIdentity);
            }
            Some(process)
        }
        _ => return Err(ControlError::Malformed("application process option")),
    };
    let database_attached = input.boolean()?;
    Ok(ControlledApplication {
        application,
        name,
        desired_running,
        state,
        incarnation,
        process,
        database_attached,
    })
}

fn decode_applications(input: &mut ResponseInput<'_>) -> Result<ControlSuccess, ControlFailure> {
    let count = usize::from(input.u16().map_err(|_| ControlFailure::Malformed)?);
    if count > MAX_APPLICATIONS {
        return Err(ControlFailure::Malformed);
    }
    let mut applications = Vec::with_capacity(count);
    for _ in 0..count {
        applications.push(decode_application(input).map_err(|_| ControlFailure::Malformed)?);
    }
    Ok(ControlSuccess::Applications(applications))
}

fn encode_invoked(
    bytes: &mut Vec<u8>,
    application: u64,
    outcome: &lkjscript_core::ExecutionOutcome,
    output: &[u8],
) -> Result<(), ControlError> {
    if output.len() > MAX_INVOKE_OUTPUT_BYTES {
        return Err(ControlError::Oversized);
    }
    let outcome = lkjscript_core::encode_execution_outcome(outcome, MAX_OUTCOME_BYTES)
        .map_err(|_| ControlError::Oversized)?;
    bytes.push(13);
    put_nonzero(bytes, application)?;
    put_u32(bytes, outcome.len())?;
    bytes.extend_from_slice(&outcome);
    put_u16(bytes, output.len())?;
    bytes.extend_from_slice(output);
    Ok(())
}

fn decode_invoked(input: &mut ResponseInput<'_>) -> Result<ControlSuccess, ControlFailure> {
    let application = input.nonzero().map_err(|_| ControlFailure::Malformed)?;
    let outcome_length = usize::try_from(input.u32().map_err(|_| ControlFailure::Malformed)?)
        .map_err(|_| ControlFailure::Malformed)?;
    if outcome_length > MAX_OUTCOME_BYTES {
        return Err(ControlFailure::Malformed);
    }
    let outcome_bytes = input
        .take(outcome_length)
        .map_err(|_| ControlFailure::Malformed)?;
    let outcome = lkjscript_core::decode_execution_outcome(outcome_bytes, MAX_OUTCOME_BYTES)
        .map_err(|_| ControlFailure::Malformed)?;
    let output_length = usize::from(input.u16().map_err(|_| ControlFailure::Malformed)?);
    if output_length > MAX_INVOKE_OUTPUT_BYTES {
        return Err(ControlFailure::Malformed);
    }
    let output = input
        .take(output_length)
        .map_err(|_| ControlFailure::Malformed)?
        .to_vec();
    Ok(ControlSuccess::ApplicationInvoked {
        application,
        outcome,
        output,
    })
}
