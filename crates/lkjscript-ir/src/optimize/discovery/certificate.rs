use crate::optimize::*;
use crate::{BlockId, FunctionId, RuntimeOp, ValueId};

pub(crate) type LegalEdit = (OptimizationEditKind, RuntimeOp, Vec<ValueId>, ValueId);

pub(crate) struct CertificateBuilder {
    records: Vec<OptimizationCertificateRecord>,
    bytes_estimate: u64,
}

impl CertificateBuilder {
    pub(crate) fn new(budget: &mut Budget) -> Result<Self, OptimizationError> {
        if CERTIFICATE_HEADER_BYTES_ESTIMATE > budget.limits.max_certificate_bytes_estimate {
            return Err(budget_error());
        }
        budget.charge(CERTIFICATE_HEADER_BYTES_ESTIMATE)?;
        Ok(Self {
            records: Vec::new(),
            bytes_estimate: CERTIFICATE_HEADER_BYTES_ESTIMATE,
        })
    }

    pub(crate) fn push(
        &mut self,
        function: FunctionId,
        block: BlockId,
        value: ValueId,
        edit: Option<LegalEdit>,
        budget: &mut Budget,
    ) -> Result<(), OptimizationError> {
        let Some((kind, operation, operands, replacement)) = edit else {
            return Ok(());
        };
        let operand_bytes = (operands.len() as u64)
            .checked_mul(4)
            .ok_or_else(budget_error)?;
        self.bytes_estimate = self
            .bytes_estimate
            .checked_add(CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE)
            .and_then(|bytes| bytes.checked_add(operand_bytes))
            .ok_or_else(budget_error)?;
        if self.records.len() as u64 >= budget.limits.max_certificate_records
            || self.bytes_estimate > budget.limits.max_certificate_bytes_estimate
        {
            return Err(budget_error());
        }
        budget.charge(
            CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE
                .checked_add(operands.len() as u64)
                .ok_or_else(budget_error)?,
        )?;
        self.records.push(OptimizationCertificateRecord {
            sequence: self.records.len() as u64,
            function,
            block,
            value,
            kind,
            expected_operation: operation,
            expected_operands: operands,
            replacement,
        });
        Ok(())
    }

    pub(crate) fn finish(self) -> Vec<OptimizationCertificateRecord> {
        self.records
    }
}
