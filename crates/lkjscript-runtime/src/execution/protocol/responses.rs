fn encode_response(output: &mut Writer, value: &ProcessResponse) -> io::Result<()> {
    match value {
        ProcessResponse::Ready { process } => {
            output.u8(3)?;
            output.u32(*process)?;
        }
        ProcessResponse::ReadyFailure { diagnostic } => {
            output.u8(4)?;
            output.text(diagnostic, MAX_DIAGNOSTIC_BYTES)?;
        }
        ProcessResponse::Outcome {
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
            ProcessResponse::Ready { process }
        }
        4 => ProcessResponse::ReadyFailure {
            diagnostic: input.text(MAX_DIAGNOSTIC_BYTES)?,
        },
        5 => {
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
