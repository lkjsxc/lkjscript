fn encode_description(
    bytes: &mut Vec<u8>,
    revision: u64,
    digest: ContractDigest,
    product: &str,
) -> Result<(), ControlError> {
    if product.is_empty() || product.len() > 64 {
        return Err(ControlError::Malformed("product identity"));
    }
    bytes.push(1);
    bytes.extend_from_slice(&revision.to_le_bytes());
    bytes.extend_from_slice(&digest.as_bytes());
    put_u16(bytes, product.len())?;
    bytes.extend_from_slice(product.as_bytes());
    Ok(())
}

fn encode_status(
    bytes: &mut Vec<u8>,
    coordinator: u64,
    clean: bool,
    sequence: u64,
    applications: u32,
) {
    bytes.push(2);
    bytes.extend_from_slice(&coordinator.to_le_bytes());
    bytes.push(u8::from(clean));
    bytes.extend_from_slice(&sequence.to_le_bytes());
    bytes.extend_from_slice(&applications.to_le_bytes());
}

fn decode_description(input: &mut ResponseInput<'_>) -> Result<ControlSuccess, ControlFailure> {
    let platform_revision = input.u64().map_err(|_| ControlFailure::Malformed)?;
    let contract_digest = ContractDigest::from_bytes(
        input
            .array::<32>()
            .map_err(|_| ControlFailure::Malformed)?,
    );
    let product = input
        .text(64)
        .map_err(|_| ControlFailure::Malformed)?;
    Ok(ControlSuccess::Description {
        platform_revision,
        contract_digest,
        product,
    })
}

fn decode_status(input: &mut ResponseInput<'_>) -> Result<ControlSuccess, ControlFailure> {
    let coordinator = input.u64().map_err(|_| ControlFailure::Malformed)?;
    let clean_shutdown = input.boolean().map_err(|_| ControlFailure::Malformed)?;
    let control_sequence = input.u64().map_err(|_| ControlFailure::Malformed)?;
    let applications = input.u32().map_err(|_| ControlFailure::Malformed)?;
    Ok(ControlSuccess::Status {
        coordinator,
        clean_shutdown,
        control_sequence,
        applications,
    })
}

fn decode_stale(input: &mut ResponseInput<'_>) -> Result<ControlSuccess, ControlFailure> {
    Err(ControlFailure::StaleRevision {
        expected: input.u64().map_err(|_| ControlFailure::Malformed)?,
        found: input.u64().map_err(|_| ControlFailure::Malformed)?,
    })
}

fn encode_failure(bytes: &mut Vec<u8>, failure: &ControlFailure) -> Result<(), ControlError> {
    match failure {
        ControlFailure::Unauthorized => bytes.push(128),
        ControlFailure::StaleRevision { expected, found } => {
            bytes.push(129);
            bytes.extend_from_slice(&expected.to_le_bytes());
            bytes.extend_from_slice(&found.to_le_bytes());
        }
        ControlFailure::ContractMismatch => bytes.push(130),
        ControlFailure::ReplayConflict => bytes.push(131),
        ControlFailure::Malformed => bytes.push(132),
        ControlFailure::Internal => bytes.push(133),
        ControlFailure::NotFound => bytes.push(134),
        ControlFailure::Rejected(message) => {
            bytes.push(135);
            put_text(bytes, message, MAX_REJECTION_BYTES)?;
        }
    }
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

fn put_u32(bytes: &mut Vec<u8>, value: usize) -> Result<(), ControlError> {
    bytes.extend_from_slice(
        &u32::try_from(value)
            .map_err(|_| ControlError::Oversized)?
            .to_le_bytes(),
    );
    Ok(())
}

fn put_text(bytes: &mut Vec<u8>, value: &str, maximum: usize) -> Result<(), ControlError> {
    if value.is_empty() || value.len() > maximum {
        return Err(ControlError::Malformed("response text"));
    }
    put_u16(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn put_nonzero(bytes: &mut Vec<u8>, value: u64) -> Result<(), ControlError> {
    if value == 0 {
        return Err(ControlError::InvalidIdentity);
    }
    bytes.extend_from_slice(&value.to_le_bytes());
    Ok(())
}
