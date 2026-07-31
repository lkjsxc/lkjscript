use super::*;

impl<J: RuntimeTier> Vm<'_, J> {
    pub(super) fn execute_failure_unwind(&mut self, top_offset: usize, include_unentered: bool) {
        let top = self.frames.len().saturating_sub(1);
        let plans: Vec<_> = self
            .frames
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(index, frame)| {
                let offset = if index == top {
                    top_offset
                } else {
                    frame.ip.saturating_sub(1)
                };
                let proto = if frame.proto == u32::MAX {
                    self.chunk.main()
                } else {
                    self.chunk.protos().get(frame.proto as usize)?
                };
                let range = proto.failure_cleanup_ranges.iter().find(|range| {
                    usize::from(range.start) <= offset && offset < usize::from(range.end)
                })?;
                let mut actions = Vec::new();
                if index == top && include_unentered {
                    if let Some(plan) = range.unentered_plan {
                        actions.extend(
                            proto
                                .failure_cleanups
                                .get(usize::from(plan))?
                                .actions
                                .clone(),
                        );
                    }
                }
                if let Some(plan) = range.plan {
                    actions.extend(
                        proto
                            .failure_cleanups
                            .get(usize::from(plan))?
                            .actions
                            .clone(),
                    );
                }
                (!actions.is_empty()).then_some((index, actions))
            })
            .collect();
        for (frame, actions) in plans {
            for action in actions {
                self.execute_failure_action(frame, action);
            }
        }
        if let Err(error) = structural_ops::cleanup_failure_roots(self) {
            self.record_failure(CleanupSubject::UniqueStorage, error.to_string());
        }
    }

    fn execute_failure_action(&mut self, frame: usize, action: FailureCleanupAction) {
        let (local, subject) = match action {
            FailureCleanupAction::EndBorrow { local, .. } => (local, CleanupSubject::UniqueStorage),
            FailureCleanupAction::DropUnique { local, .. } => {
                (local, CleanupSubject::UniqueStorage)
            }
            FailureCleanupAction::DropResource { local, kind, .. } => {
                (local, CleanupSubject::Resource(kind))
            }
            FailureCleanupAction::EndStructuralBorrow { local, .. }
            | FailureCleanupAction::DropStructural { local, .. }
            | FailureCleanupAction::AbortStructuralDestination { local, .. } => {
                (local, CleanupSubject::UniqueStorage)
            }
        };
        if let Some(result) = structural_ops::cleanup_failure_action(self, frame, action) {
            if let Err(error) = result {
                self.record_failure(subject, error.to_string());
            }
            return;
        }
        let Some(index) = self
            .frames
            .get(frame)
            .and_then(|frame| frame.locals_base.checked_add(usize::from(local)))
        else {
            self.record_failure(subject, "VM failure cleanup lost its frame local".into());
            return;
        };
        let Some(slot) = self.stack.get_mut(index) else {
            self.record_failure(subject, "VM failure cleanup local is out of range".into());
            return;
        };
        let value = *slot;
        if value.is_invalid() {
            return;
        }
        *slot = Value::INVALID;
        if let FailureCleanupAction::DropResource { kind, .. } = action {
            if let Some(result) = structural_ops::drop_resource_adapter(self, value, kind) {
                if let Err(error) = result {
                    self.record_failure(subject, error.to_string());
                }
                return;
            }
        }
        let result = match action {
            FailureCleanupAction::EndBorrow { .. } if structural_ops::is_byte_view(self, value) => {
                structural_ops::end_byte_view(self, value)
            }
            FailureCleanupAction::EndBorrow { .. } => self.unique.end_borrow(value),
            FailureCleanupAction::DropUnique { place, .. } => {
                let result = self.unique.drop_owner(value);
                if let Some(place) = place {
                    if let Some(target) = self
                        .frames
                        .get_mut(frame)
                        .and_then(|frame| frame.unique_places.get_mut(usize::from(place)))
                    {
                        *target = unique::RuntimePlace::Inactive;
                    }
                }
                result
            }
            FailureCleanupAction::DropResource { kind, .. } => match kind {
                lkjscript_core::ResourceKind::SqliteConnection => {
                    self.resources.sqlite_close(value).map(|_| ())
                }
                lkjscript_core::ResourceKind::SqliteStatement => {
                    self.resources.sqlite_finalize(value).map(|_| ())
                }
                _ => self.resources.close(value).map(|_| ()),
            },
            FailureCleanupAction::EndStructuralBorrow { .. }
            | FailureCleanupAction::DropStructural { .. }
            | FailureCleanupAction::AbortStructuralDestination { .. } => {
                Err(Error::msg("structural failure cleanup dispatch mismatch"))
            }
        };
        if let Err(error) = result {
            self.record_failure(subject, error.to_string());
        }
    }

    fn record_failure(&mut self, subject: CleanupSubject, message: String) {
        self.cleanup_failures
            .push(CleanupPhase::Ordinary, subject, message);
    }

    pub(super) fn restore_structural_handoffs(&mut self) {
        if let Err(error) = structural_ops::restore_handoffs(self) {
            self.record_failure(CleanupSubject::UniqueStorage, error.to_string());
        }
    }

    pub(super) fn current_failure_offset(&self) -> usize {
        self.frames.last().map_or(0, |frame| frame.ip)
    }

    pub(super) fn is_failure_boundary(&self, offset: usize) -> bool {
        let Some(frame) = self.frames.last() else {
            return true;
        };
        let proto = if frame.proto == u32::MAX {
            self.chunk.main()
        } else {
            let Some(proto) = self.chunk.protos().get(frame.proto as usize) else {
                return true;
            };
            proto
        };
        proto
            .failure_cleanup_ranges
            .iter()
            .find(|range| usize::from(range.start) <= offset && offset < usize::from(range.end))
            .is_none_or(|range| usize::from(range.start) == offset)
    }
}
