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
        let mut unplaced = self
            .unplaced_owners
            .iter()
            .copied()
            .filter(|value| !exclusions.contains(value))
            .collect::<Vec<_>>();
        unplaced.sort();
        for value in unplaced.into_iter().rev() {
            let glue = match self.value_type(value)? {
                SsaType::ByteVector => DropGlueIdentity::ByteVector,
                SsaType::Bytes => DropGlueIdentity::Bytes,
                SsaType::Resource(kind) => DropGlueIdentity::Resource(kind),
                ty
                @ (SsaType::Str | SsaType::Path | SsaType::Product(_) | SsaType::Enum { .. }) => {
                    structural_glue(self.structural, &ty)?
                }
                SsaType::StructuralDestination(type_id) => {
                    let item = self
                        .structural
                        .types
                        .get(type_id.index().unwrap_or(usize::MAX))
                        .filter(|item| item.id == type_id)
                        .ok_or_else(|| {
                            Error::msg("SSA destination owner has no exact drop metadata")
                        })?;
                    DropGlueIdentity::Structural(StructuralDropGlueIdentity::Destination {
                        type_id,
                        layout: item.layout,
                    })
                }
                SsaType::Unit
                | SsaType::Bool
                | SsaType::I64
                | SsaType::F64
                | SsaType::Symbol
                | SsaType::ByteSlice
                | SsaType::ByteSliceMut
                | SsaType::Capability(_)
                | SsaType::List(_)
                | SsaType::Function(_)
                | SsaType::TypeParameter(_) => {
                    return Err(Error::msg("SSA unplaced owner has no exact drop glue"))
                }
            };
            actions.push(FailureCleanupAction::DropOwner {
                place: None,
                value,
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
