fn encode_response(output: &mut Writer, value: &ProcessResponse) -> io::Result<()> {
    match value {
        ProcessResponse::Ready {
            process,
            provenance,
        } => {
            output.u8(3)?;
            output.u32(*process)?;
            encode_provenance(output, provenance)?;
        }
        ProcessResponse::ReadyFailure { diagnostic } => {
            output.u8(4)?;
            output.text(diagnostic, MAX_DIAGNOSTIC_BYTES)?;
        }
        ProcessResponse::Outcome {
            provenance,
            cell,
            outcome,
            output: application_output,
            flushes,
        } => {
            if application_output.len() > MAX_APPLICATION_OUTPUT_BYTES {
                return Err(invalid("application output exceeds process bound"));
            }
            if *flushes > MAX_FLUSHES {
                return Err(invalid("application flush count exceeds process bound"));
            }
            let outcome =
                lkjscript_core::encode_execution_outcome(outcome, PROCESS_OUTCOME_CODEC_LIMITS)
                .map_err(|error| invalid(error.to_string()))?;
            output.u8(5)?;
            encode_provenance(output, provenance)?;
            output.u64(nonzero(*cell, "execution cell")?)?;
            output.bytes(&outcome)?;
            output.bytes(application_output)?;
            output.u64(*flushes)?;
        }
        ProcessResponse::Stopped => output.u8(6)?,
    }
    Ok(())
}

fn decode_response(input: &mut Reader<'_>) -> io::Result<ProcessResponse> {
    Ok(match input.u8()? {
        3 => {
            let process = input.u32()?;
            if process == 0 {
                return Err(invalid("worker process identity must be nonzero"));
            }
            ProcessResponse::Ready {
                process,
                provenance: decode_provenance(input)?,
            }
        }
        4 => ProcessResponse::ReadyFailure {
            diagnostic: input.text(MAX_DIAGNOSTIC_BYTES)?,
        },
        5 => {
            let provenance = decode_provenance(input)?;
            let cell = nonzero(input.u64()?, "execution cell")?;
            let encoded = input.bytes(MAX_FRAME_BYTES)?;
            let outcome =
                lkjscript_core::decode_execution_outcome(encoded, PROCESS_OUTCOME_CODEC_LIMITS)
                .map_err(|error| invalid(error.to_string()))?;
            let output = input.bytes(MAX_APPLICATION_OUTPUT_BYTES)?.to_vec();
            let flushes = input.u64()?;
            if flushes > MAX_FLUSHES {
                return Err(invalid("application flush count exceeds process bound"));
            }
            ProcessResponse::Outcome {
                provenance,
                cell,
                outcome,
                output,
                flushes,
            }
        }
        6 => ProcessResponse::Stopped,
        _ => return Err(invalid("unknown process response")),
    })
}

fn encode_provenance(
    output: &mut Writer,
    value: &ProcessProgramProvenance,
) -> io::Result<()> {
    output.u64(nonzero(value.platform_revision, "outcome platform revision")?)?;
    if value.contract == [0; 32] {
        return Err(invalid("outcome provenance digest must be nonzero"));
    }
    output.extend(&value.contract)?;
    output.u64(nonzero(value.application, "outcome application")?)?;
    output.u64(nonzero(value.incarnation, "outcome incarnation")?)?;
    for digest in [
        value.package,
        value.entry,
        value.prepared.bytes(),
        value.return_semantic,
        value.root_witness_group,
        value.root_witness_member,
    ] {
        if digest == [0; 32] {
            return Err(invalid("outcome provenance digest must be nonzero"));
        }
        output.extend(&digest)?;
    }
    Ok(())
}

fn decode_provenance(input: &mut Reader<'_>) -> io::Result<ProcessProgramProvenance> {
    let platform_revision = nonzero(input.u64()?, "outcome platform revision")?;
    let contract = digest(input)?;
    let application = nonzero(input.u64()?, "outcome application")?;
    let incarnation = nonzero(input.u64()?, "outcome incarnation")?;
    Ok(ProcessProgramProvenance {
        platform_revision,
        contract,
        application,
        incarnation,
        package: digest(input)?,
        entry: digest(input)?,
        prepared: PreparedProgramIdentity::new(digest(input)?)
            .map_err(|error| invalid(error.to_string()))?,
        return_semantic: digest(input)?,
        root_witness_group: digest(input)?,
        root_witness_member: digest(input)?,
    })
}

fn digest(input: &mut Reader<'_>) -> io::Result<[u8; 32]> {
    let value = input
        .take(32)?
        .try_into()
        .map_err(|_| invalid("outcome provenance digest length"))?;
    if value == [0; 32] {
        Err(invalid("outcome provenance digest must be nonzero"))
    } else {
        Ok(value)
    }
}
