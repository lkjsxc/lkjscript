use lkjscript_core::ExecutionOutcome;

#[derive(Debug)]
pub(crate) struct ProcessInvokeFailure {
    message: String,
    fatal: bool,
}

impl ProcessInvokeFailure {
    pub(crate) fn recoverable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            fatal: false,
        }
    }

    pub(crate) fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            fatal: true,
        }
    }

    pub(crate) const fn is_fatal(&self) -> bool {
        self.fatal
    }

    pub(crate) fn into_message(self) -> String {
        self.message
    }
}

impl ProcessCell {
    pub(crate) fn invoke(
        &mut self,
        cell: u64,
        arguments: Vec<String>,
    ) -> Result<
        (ExecutionOutcome, Vec<u8>, u64, Option<crate::RehydrationReport>),
        ProcessInvokeFailure,
    > {
        self.ensure_running().map_err(ProcessInvokeFailure::fatal)?;
        if let Err(error) = write_request(
            &mut self.input,
            &ProcessRequest::Invoke { cell, arguments },
        ) {
            let message = self.fail(format!("write process invocation: {error}"));
            return Err(ProcessInvokeFailure::fatal(message));
        }
        let response = match self.read() {
            Ok(response) => response,
            Err(error) => {
                let message = self.fail(error);
                return Err(ProcessInvokeFailure::fatal(message));
            }
        };
        let ProcessResponse::Outcome {
            provenance,
            cell: received,
            outcome,
            output,
            flushes,
        } = response
        else {
            let message = self.fail("unexpected process invocation response".into());
            return Err(ProcessInvokeFailure::fatal(message));
        };
        if received != cell || provenance != self.provenance {
            return Err(ProcessInvokeFailure::recoverable(
                "process outcome provenance mismatch",
            ));
        }
        let (outcome, report) = crate::rehydrate_process_outcome(
            outcome,
            self.parent_chunk.as_ref(),
            self.prepared,
        )
        .map_err(ProcessInvokeFailure::recoverable)?;
        self.last_rehydration = report;
        Ok((outcome, output, flushes, report))
    }
}
