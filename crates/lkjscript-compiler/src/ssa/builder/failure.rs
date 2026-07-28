use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn intern_failure_cleanup(
        &mut self,
        exclusions: &[ValueId],
    ) -> Result<Option<FailureCleanupId>> {
        let mut actions = Vec::new();
        for (loan, active) in self.active_loans.iter().rev() {
            actions.push(FailureCleanupAction::EndBorrow {
                place: active.place,
                loan: *loan,
                kind: active.kind,
                value: active.value,
            });
        }
        for value in self
            .unplaced_owners
            .iter()
            .rev()
            .filter(|value| !exclusions.contains(value))
        {
            let glue = match self.value_type(*value)? {
                SsaType::ByteVector => DropGlueIdentity::ByteVector,
                SsaType::Bytes => DropGlueIdentity::Bytes,
                SsaType::Resource(kind) => DropGlueIdentity::Resource(kind),
                _ => return Err(Error::msg("SSA unplaced owner has no exact drop glue")),
            };
            actions.push(FailureCleanupAction::DropOwner {
                place: None,
                value: *value,
                glue,
            });
        }
        for place in self.places.iter().rev() {
            let Some(glue) = place.drop_glue else {
                continue;
            };
            let binding = BindingId::new(place.binding.raw());
            let Some(value) = self.env.get(&binding).copied() else {
                continue;
            };
            actions.push(FailureCleanupAction::DropOwner {
                place: Some(place.id),
                value,
                glue,
            });
        }
        if actions.is_empty() {
            return Ok(None);
        }
        if let Some(existing) = self
            .failure_cleanups
            .iter()
            .find(|existing| existing.actions == actions)
        {
            return Ok(Some(existing.id));
        }
        let raw = u32::try_from(self.failure_cleanups.len())
            .map_err(|_| Error::msg("SSA failure-cleanup plan count exceeds u32"))?;
        let id = FailureCleanupId::new(raw);
        self.failure_cleanups
            .push(FailureCleanupPlan { id, actions });
        Ok(Some(id))
    }
}
