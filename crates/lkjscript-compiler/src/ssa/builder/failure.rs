use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn intern_failure_cleanup(
        &mut self,
        exclusions: &[ValueId],
    ) -> Result<Option<FailureCleanupRoots>> {
        let exclusions: HashSet<_> = exclusions.iter().copied().collect();
        let mut places = None;

        // Cleanup order is loans, unplaced owners, then places, each in reverse
        // acquisition order. Build the immutable chain in the opposite direction
        // so every new node links only to an already-interned prior node.
        for place in &self.places {
            let Some(glue) = place.drop_glue else {
                continue;
            };
            let binding = BindingId::new(place.binding.raw());
            let Some(value) = self.env.get(&binding).copied() else {
                continue;
            };
            places = Some(
                self.failure_cleanups
                    .intern(
                        FailureCleanupAction::DropOwner {
                            place: Some(place.id),
                            value,
                            glue,
                        },
                        places,
                    )
                    .map_err(|error| Error::msg(error.to_string()))?,
            );
        }

        let mut unplaced = self
            .unplaced_owners
            .iter()
            .copied()
            .filter(|value| !exclusions.contains(value))
            .collect::<Vec<_>>();
        unplaced.sort_unstable();
        unplaced.dedup();
        let mut unplaced_root = None;
        for value in unplaced {
            let glue = self.failure_drop_glue(value)?;
            unplaced_root = Some(
                self.failure_cleanups
                    .intern(
                        FailureCleanupAction::DropOwner {
                            place: None,
                            value,
                            glue,
                        },
                        unplaced_root,
                    )
                    .map_err(|error| Error::msg(error.to_string()))?,
            );
        }

        let mut loans = None;
        for (loan, active) in &self.active_loans {
            loans = Some(
                self.failure_cleanups
                    .intern(
                        FailureCleanupAction::EndBorrow {
                            place: active.place,
                            loan: *loan,
                            kind: active.kind,
                            value: active.value,
                        },
                        loans,
                    )
                    .map_err(|error| Error::msg(error.to_string()))?,
            );
        }
        if loans.is_none() && unplaced_root.is_none() && places.is_none() {
            Ok(None)
        } else {
            Ok(Some(FailureCleanupRoots {
                loans,
                unplaced: unplaced_root,
                places,
            }))
        }
    }

    fn failure_drop_glue(&self, value: ValueId) -> Result<DropGlueIdentity> {
        match self.value_type(value)? {
            SsaType::ByteVector => Ok(DropGlueIdentity::ByteVector),
            SsaType::Bytes => Ok(DropGlueIdentity::Bytes),
            SsaType::Resource(kind) => Ok(DropGlueIdentity::Resource(kind)),
            ty @ (SsaType::Str | SsaType::Path | SsaType::Product(_) | SsaType::Enum { .. }) => {
                structural_glue(self.structural, &ty)
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
                Ok(DropGlueIdentity::Structural(
                    StructuralDropGlueIdentity::Destination {
                        type_id,
                        layout: item.layout,
                    },
                ))
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
                Err(Error::msg("SSA unplaced owner has no exact drop glue"))
            }
        }
    }
}
