use super::*;

impl Vm<'_> {
    pub(super) fn execute_failure_unwind(&mut self, top_offset: usize, include_unentered: bool) {
        let top = self.frames.len().saturating_sub(1);
        let roots: Vec<_> = self
            .frames
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(index, frame)| {
                let offset = if index == top {
                    top_offset
                } else {
                    frame.instruction_offset
                };
                let offset = u64::try_from(offset).ok()?;
                let proto = if let Some(prototype) = frame.proto {
                    self.chunk.protos().get(prototype)?
                } else {
                    self.chunk.main()
                };
                let range = lkjscript_core::failure_cleanup_range_at(
                    &proto.failure_cleanup_ranges,
                    offset,
                )?;
                Some((
                    index,
                    frame.proto,
                    (index == top && include_unentered)
                        .then_some(range.unentered_plan)
                        .flatten(),
                    range.plan,
                ))
            })
            .collect();
        for (frame, proto, unentered, ordinary) in roots {
            self.execute_failure_chain(frame, proto, unentered);
            for root in ordinary
                .into_iter()
                .flat_map(lkjscript_core::FailureCleanupRoots::ids)
            {
                self.execute_failure_chain(frame, proto, Some(root));
            }
        }
        if let Err(error) = structural_ops::cleanup_failure_roots(self) {
            self.record_failure(CleanupSubject::UniqueStorage, error.to_string());
        }
    }

    fn execute_failure_chain(
        &mut self,
        frame: usize,
        prototype: Option<usize>,
        mut root: Option<lkjscript_core::FailureCleanupId>,
    ) {
        while let Some(id) = root {
            let Some(index) = id.index() else {
                self.record_failure(
                    CleanupSubject::UniqueStorage,
                    "validated VM failure-cleanup ID exceeds host usize".into(),
                );
                return;
            };
            let node = if let Some(prototype) = prototype {
                self.chunk
                    .protos()
                    .get(prototype)
                    .and_then(|proto| proto.failure_cleanups.get(index))
                    .copied()
            } else {
                self.chunk.main().failure_cleanups.get(index).copied()
            };
            let Some(node) = node else {
                self.record_failure(
                    CleanupSubject::UniqueStorage,
                    "validated VM failure-cleanup chain lost a node".into(),
                );
                return;
            };
            root = node.next;
            self.execute_failure_action(frame, node.action);
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
            .and_then(|frame| frame.locals_base.checked_add(local))
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
                        .and_then(|frame| frame.unique_places.get_mut(place))
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

    pub(super) fn cleanup_unentered_call_arguments(
        &mut self,
        proto: &lkjscript_core::FunctionProto,
        arguments: &[Value],
    ) {
        for (index, value) in arguments.iter().copied().enumerate().rev() {
            self.cleanup_unentered_call_argument(proto, index, value);
        }
    }

    pub(super) fn cleanup_unentered_call_stack_arguments(
        &mut self,
        proto: &lkjscript_core::FunctionProto,
        start: usize,
        count: usize,
    ) {
        for index in (0..count).rev() {
            let value = start
                .checked_add(index)
                .and_then(|slot| self.stack.get(slot))
                .copied();
            if let Some(value) = value {
                self.cleanup_unentered_call_argument(proto, index, value);
            } else {
                self.record_failure(
                    CleanupSubject::UniqueStorage,
                    "failed call argument cleanup lost its operand".into(),
                );
            }
        }
    }

    fn cleanup_unentered_call_argument(
        &mut self,
        proto: &lkjscript_core::FunctionProto,
        index: usize,
        value: Value,
    ) {
        let cleanup = if proto
            .parameter_unique_places
            .get(index)
            .copied()
            .flatten()
            .is_some()
        {
            Some((CleanupSubject::UniqueStorage, self.unique.drop_owner(value)))
        } else if let Some(kind) = proto
            .parameter_resources
            .get(index)
            .copied()
            .flatten()
            .filter(|_| {
                proto
                    .parameter_resource_places
                    .get(index)
                    .copied()
                    .flatten()
                    .is_some()
            })
        {
            let result =
                if let Some(result) = structural_ops::drop_resource_adapter(self, value, kind) {
                    result
                } else {
                    match kind {
                        lkjscript_core::ResourceKind::SqliteConnection => {
                            self.resources.sqlite_close(value).map(|_| ())
                        }
                        lkjscript_core::ResourceKind::SqliteStatement => {
                            self.resources.sqlite_finalize(value).map(|_| ())
                        }
                        _ => self.resources.close(value).map(|_| ()),
                    }
                };
            Some((CleanupSubject::Resource(kind), result))
        } else if structural_parameter_is_owned(self, proto, index, value) {
            Some((
                CleanupSubject::UniqueStorage,
                structural_ops::drop_registered_owner(self, value),
            ))
        } else {
            None
        };
        if let Some((subject, Err(error))) = cleanup {
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

    pub(super) fn is_failure_boundary(&mut self, offset: usize) -> bool {
        let Some(offset) = u64::try_from(offset).ok() else {
            return true;
        };
        let chunk = self.chunk;
        let Some(frame) = self.frames.last_mut() else {
            return true;
        };
        let proto = if let Some(prototype) = frame.proto {
            let Some(proto) = chunk.protos().get(prototype) else {
                return true;
            };
            proto
        } else {
            chunk.main()
        };
        failure_cleanup_range_with_cursor(
            &proto.failure_cleanup_ranges,
            offset,
            &mut frame.failure_cleanup_cursor,
        )
        .is_none_or(|range| range.start == offset)
    }
}

fn structural_parameter_is_owned(
    vm: &Vm<'_>,
    proto: &lkjscript_core::FunctionProto,
    index: usize,
    value: Value,
) -> bool {
    proto
        .parameter_structural_places
        .get(index)
        .copied()
        .flatten()
        .is_some()
        || (proto.parameter_requires_independent_owner(index)
            && structural_ops::owner_handoff_is_pending(vm, value))
}

fn failure_cleanup_range_with_cursor<'a>(
    ranges: &'a [lkjscript_core::FailureCleanupRange],
    offset: u64,
    cursor: &mut usize,
) -> Option<&'a lkjscript_core::FailureCleanupRange> {
    if *cursor > ranges.len()
        || (*cursor > 0
            && ranges
                .get(*cursor - 1)
                .is_some_and(|range| range.end > offset))
    {
        *cursor = ranges.partition_point(|range| range.end <= offset);
    } else if ranges.get(*cursor).is_some_and(|range| range.end <= offset) {
        let next = cursor.saturating_add(1);
        *cursor =
            if next == ranges.len() || ranges.get(next).is_some_and(|range| range.end > offset) {
                next
            } else {
                ranges.partition_point(|range| range.end <= offset)
            };
    }
    ranges
        .get(*cursor)
        .filter(|range| range.start <= offset && offset < range.end)
}

#[cfg(test)]
mod range_cursor_tests {
    use super::failure_cleanup_range_with_cursor;

    fn range(start: u64, end: u64) -> lkjscript_core::FailureCleanupRange {
        lkjscript_core::FailureCleanupRange {
            start,
            end,
            plan: None,
            unentered_plan: None,
        }
    }

    #[test]
    fn frame_cursor_matches_indexed_lookup_across_steps_gaps_and_backedges() {
        let ranges = [range(1, 3), range(3, 7), range(10, 12), range(15, 20)];
        let sequences = [
            (0..=21).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 6, 7, 10, 11, 15, 19, 20, 8, 3, 14, 4, 0],
            vec![15, 15, 16, 2, 2, 12, 10, u64::MAX, 1],
        ];
        for offsets in sequences {
            let mut cursor = 0;
            for offset in offsets {
                assert_eq!(
                    failure_cleanup_range_with_cursor(&ranges, offset, &mut cursor),
                    lkjscript_core::failure_cleanup_range_at(&ranges, offset),
                );
                assert!(cursor <= ranges.len());
            }
        }
    }
}
