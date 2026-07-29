impl CleanupFailure {
    pub(crate) fn from_wire_parts(
        phase: CleanupPhase,
        subject: CleanupSubject,
        message: String,
        omitted_message_bytes: usize,
    ) -> Self {
        Self {
            phase,
            subject,
            message,
            omitted_message_bytes,
        }
    }
}

impl CleanupFailures {
    pub(crate) fn from_wire_parts(
        limits: CleanupFailureLimits,
        retained: Vec<CleanupFailure>,
        retained_message_bytes: usize,
        omitted_message_bytes: usize,
        omitted_failures: usize,
    ) -> crate::Result<Self> {
        let actual_retained = retained.iter().try_fold(0_usize, |total, failure| {
            total.checked_add(failure.message.len())
        });
        let actual_omitted = retained.iter().try_fold(0_usize, |total, failure| {
            total.checked_add(failure.omitted_message_bytes)
        });
        if retained.len() > limits.max_failures
            || actual_retained != Some(retained_message_bytes)
            || retained_message_bytes > limits.max_message_bytes
            || actual_omitted.is_none_or(|bytes| bytes > omitted_message_bytes)
        {
            return Err(crate::Error::msg("invalid cleanup failure wire accounting"));
        }
        Ok(Self {
            limits,
            retained,
            retained_message_bytes,
            omitted_message_bytes,
            omitted_failures,
        })
    }

    pub const fn limits(&self) -> CleanupFailureLimits {
        self.limits
    }
}
