const MAX_SESSIONS: usize = 64;

fn encode_session(bytes: &mut Vec<u8>, session: &ControlledSession) -> Result<(), ControlError> {
    put_nonzero(bytes, session.session)?;
    if session.broker_instance == [0; 32] || session.process == 0 {
        return Err(ControlError::InvalidIdentity);
    }
    bytes.extend_from_slice(&session.broker_instance);
    bytes.extend_from_slice(&session.process.to_le_bytes());
    bytes.extend_from_slice(&session.user.to_le_bytes());
    bytes.extend_from_slice(&session.group.to_le_bytes());
    bytes.push(match session.backend {
        super::SessionBackend::None => 0,
    });
    bytes.extend_from_slice(&session.lease_deadline.to_le_bytes());
    Ok(())
}

fn decode_session(input: &mut ResponseInput<'_>) -> Result<ControlledSession, ControlError> {
    let session = input.nonzero()?;
    let broker_instance = input.array::<32>()?;
    let process = input.u32()?;
    if broker_instance == [0; 32] || process == 0 {
        return Err(ControlError::InvalidIdentity);
    }
    let user = input.u32()?;
    let group = input.u32()?;
    let backend = match input.u8()? {
        0 => super::SessionBackend::None,
        _ => return Err(ControlError::Malformed("session backend")),
    };
    let lease_deadline = input.u64()?;
    Ok(ControlledSession {
        session,
        broker_instance,
        process,
        user,
        group,
        backend,
        lease_deadline,
    })
}

fn encode_sessions(
    bytes: &mut Vec<u8>,
    sessions: &[ControlledSession],
) -> Result<(), ControlError> {
    if sessions.len() > MAX_SESSIONS {
        return Err(ControlError::Oversized);
    }
    bytes.push(21);
    put_u16(bytes, sessions.len())?;
    for session in sessions {
        encode_session(bytes, session)?;
    }
    Ok(())
}

fn decode_sessions(input: &mut ResponseInput<'_>) -> Result<ControlSuccess, ControlFailure> {
    let count = usize::from(input.u16().map_err(|_| ControlFailure::Malformed)?);
    if count > MAX_SESSIONS {
        return Err(ControlFailure::Malformed);
    }
    let mut sessions = Vec::with_capacity(count);
    for _ in 0..count {
        sessions.push(decode_session(input).map_err(|_| ControlFailure::Malformed)?);
    }
    Ok(ControlSuccess::Sessions(sessions))
}
