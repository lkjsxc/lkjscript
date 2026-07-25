use crate::optimize::*;

pub(crate) struct Budget {
    pub(crate) limits: OptimizationLimits,
    pub(crate) work: u64,
    pub(crate) iterations: u64,
    pub(crate) input_instructions: Option<u64>,
    pub(crate) discovery_passes: u64,
    pub(crate) checker_passes: u64,
    pub(crate) reconstruction_passes: u64,
    pub(crate) cleanup_passes: u64,
    pub(crate) validation_passes: u64,
}

impl Budget {
    pub(crate) const fn new(limits: OptimizationLimits) -> Self {
        Self {
            limits,
            work: 0,
            iterations: 0,
            input_instructions: None,
            discovery_passes: 0,
            checker_passes: 0,
            reconstruction_passes: 0,
            cleanup_passes: 0,
            validation_passes: 0,
        }
    }

    pub(crate) fn charge(&mut self, amount: u64) -> Result<(), OptimizationError> {
        self.work = self.work.checked_add(amount).ok_or_else(budget_error)?;
        if self.work > self.limits.max_work_units {
            return Err(budget_error());
        }
        Ok(())
    }

    pub(crate) fn set_input_instructions(&mut self, instructions: u64) {
        self.input_instructions = Some(instructions);
    }

    pub(crate) fn charge_cleanup_pass(&mut self) -> Result<(), OptimizationError> {
        self.iterations = self.iterations.checked_add(1).ok_or_else(budget_error)?;
        if self.iterations > self.limits.max_iterations {
            return Err(budget_error());
        }
        self.cleanup_passes = self.cleanup_passes.saturating_add(1);
        Ok(())
    }

    pub(crate) fn check_growth(&self, instructions: u64) -> Result<(), OptimizationError> {
        let input = self.input_instructions.unwrap_or(instructions);
        if instructions.saturating_sub(input) > self.limits.max_instruction_growth {
            return Err(budget_error());
        }
        Ok(())
    }

    pub(crate) fn charge_validation(
        &mut self,
        shape: &ProgramShape,
    ) -> Result<(), OptimizationError> {
        self.validation_passes = self.validation_passes.saturating_add(1);
        self.charge(shape.validation_units())
    }
}

pub(crate) fn budget_error() -> OptimizationError {
    OptimizationError::new(
        OptimizationFailureCode::BudgetExceeded,
        "optimization work, shape, certificate, growth, or iteration budget exceeded",
    )
}

pub(crate) const CERTIFICATE_HEADER_BYTES_ESTIMATE: u64 = 8;
pub(crate) const CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE: u64 = 31;

pub(crate) fn certificate_size_estimate(
    certificate: &OptimizationCertificate,
) -> Result<u64, OptimizationError> {
    certificate
        .records
        .iter()
        .try_fold(CERTIFICATE_HEADER_BYTES_ESTIMATE, |total, record| {
            let operands = u64::try_from(record.expected_operands.len())
                .map_err(|_| budget_error())?
                .checked_mul(4)
                .ok_or_else(budget_error)?;
            total
                .checked_add(CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE)
                .and_then(|value| value.checked_add(operands))
                .ok_or_else(budget_error)
        })
}

pub(crate) fn preflight_certificate(
    certificate: &OptimizationCertificate,
    budget: &mut Budget,
) -> Result<(), OptimizationError> {
    let records = u64::try_from(certificate.records.len()).map_err(|_| budget_error())?;
    if records > budget.limits.max_certificate_records {
        return Err(budget_error());
    }
    let bytes = certificate_size_estimate(certificate)?;
    if bytes > budget.limits.max_certificate_bytes_estimate {
        return Err(budget_error());
    }
    budget.charge(CERTIFICATE_HEADER_BYTES_ESTIMATE)?;
    for record in &certificate.records {
        let operands = u64::try_from(record.expected_operands.len()).map_err(|_| budget_error())?;
        budget.charge(
            CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE
                .checked_add(operands)
                .ok_or_else(budget_error)?,
        )?;
    }
    Ok(())
}
