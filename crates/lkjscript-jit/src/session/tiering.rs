use crate::*;

impl JitSession {
    pub fn observe_function_entry(&mut self, prototype: u64) -> EntryDecision {
        let Some(function) = self.function_for_prototype(prototype) else {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        };
        let Some(index) = function.index() else {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        };
        let Some(record) = self.functions.get_mut(index) else {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        };
        record.call_count = record.call_count.saturating_add(1);
        if !record.auto_entry_eligible {
            // A supported compilation group may contain reference-signature
            // helpers for direct generated calls. Until VM/native reference
            // adapters exist, those helpers are never eligible VM entries.
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        }
        if record.state == TierState::BaselineNative {
            return EntryDecision::Native(function);
        }
        if !self.config.auto_enabled {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        }
        if record.epoch != self.config.epoch {
            record.epoch = self.config.epoch;
            if record.attempts < self.config.max_attempts_per_function {
                record.state = TierState::Observed;
                record.last_failure = None;
            }
        }
        if matches!(
            record.state,
            TierState::Disabled | TierState::BaselineCompiling
        ) || record.last_failure.is_some() && record.epoch == self.config.epoch
        {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        }
        if record.state == TierState::VmOnly {
            record.state = TierState::Observed;
        }
        if record.call_count < self.config.auto_threshold.max(1) {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        }

        record.state = TierState::BaselineCompiling;
        record.attempts = record.attempts.saturating_add(1);
        match self.compile_group(function) {
            Ok(_) => {}
            Err(error) => {
                self.compile_failures = self.compile_failures.saturating_add(1);
                if let Some(record) = self.functions.get_mut(index) {
                    record.last_failure = Some(error.code());
                    record.state = if matches!(
                        error.code(),
                        FailureCode::UnsupportedType
                            | FailureCode::UnsupportedOperation
                            | FailureCode::UnsupportedSignature
                            | FailureCode::IndirectCall
                            | FailureCode::RecursionUnsupported
                    ) || record.attempts >= self.config.max_attempts_per_function
                    {
                        TierState::Disabled
                    } else {
                        TierState::Observed
                    };
                }
            }
        }
        // Compilation at this entry is for later calls; this invocation remains
        // in the VM and is never mislabeled as OSR.
        self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
        EntryDecision::Interpret
    }

    pub fn scalar_signature(&self, function: FunctionId) -> Option<ScalarSignature> {
        let signature: &IrSignature = function
            .index()
            .and_then(|index| self.program.program().functions.get(index))
            .filter(|item| item.id == function)
            .map(|item| &item.signature)?;
        let parameters = signature
            .parameters
            .iter()
            .map(native_type)
            .collect::<Option<Vec<_>>>()?;
        let result = native_type(&signature.result)?;
        Some(ScalarSignature { parameters, result })
    }

    pub fn trap_message_for(
        &self,
        function: FunctionId,
        trap: TrapCode,
        site: Option<u64>,
    ) -> String {
        self.trap_message(function, trap, site)
    }
}
