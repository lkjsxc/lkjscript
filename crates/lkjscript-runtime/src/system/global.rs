pub(crate) struct GlobalAdmission {
    pub(crate) next_ticket: u64,
    pub(crate) serving_ticket: u64,
    pub(crate) active: usize,
    pub(crate) total: u64,
    pub(crate) peak: usize,
    pub(crate) limits: crate::RuntimeLimits,
}

impl GlobalAdmission {
    fn new(limits: crate::RuntimeLimits) -> Self {
        Self {
            next_ticket: 0,
            serving_ticket: 0,
            active: 0,
            total: 0,
            peak: 0,
            limits,
        }
    }

    pub(crate) fn advance(&mut self) {
        self.serving_ticket = self.serving_ticket.saturating_add(1);
    }

    pub(crate) fn admitted(&mut self) {
        self.advance();
        self.active += 1;
        self.total += 1;
        self.peak = self.peak.max(self.active);
    }

    pub(crate) fn complete(&mut self) {
        self.active = self.active.saturating_sub(1);
    }

    pub(crate) fn accounting(&self) -> crate::RuntimeAccounting {
        crate::RuntimeAccounting {
            active_invocations: self.active,
            total_invocations: self.total,
            peak_concurrent: self.peak,
            limits: self.limits,
        }
    }
}
