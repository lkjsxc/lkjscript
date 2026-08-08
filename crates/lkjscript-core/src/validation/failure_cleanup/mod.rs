use std::collections::HashSet;

use super::{Kind, OwnerIdentity, State, UniquePlaceState};
use crate::{Error, FailureCleanupAction, FunctionProto, Result};
use checks::{
    local_owner, validate_loan, validate_structural_drop, validate_structural_loan,
    validate_unique_drop,
};

mod checks;

pub(super) struct RangeCursor<'a> {
    ranges: &'a [crate::FailureCleanupRange],
    index: usize,
}

impl<'a> RangeCursor<'a> {
    pub(super) fn new(proto: &'a FunctionProto, offset: usize) -> Result<Self> {
        let offset = u64::try_from(offset)
            .map_err(|_| Error::msg("bytecode instruction offset exceeds cleanup range width"))?;
        Ok(Self {
            ranges: &proto.failure_cleanup_ranges,
            index: proto
                .failure_cleanup_ranges
                .partition_point(|range| range.end <= offset),
        })
    }

    fn covering(&mut self, offset: u64) -> Option<&'a crate::FailureCleanupRange> {
        while self
            .ranges
            .get(self.index)
            .is_some_and(|range| range.end <= offset)
        {
            self.index += 1;
        }
        self.ranges
            .get(self.index)
            .filter(|range| range.start <= offset && offset < range.end)
    }
}

pub(super) fn validate_at_offset(
    proto: &FunctionProto,
    offset: usize,
    state: &State,
    ranges: &mut RangeCursor<'_>,
) -> Result<()> {
    let host_offset = offset;
    let offset = u64::try_from(host_offset)
        .map_err(|_| Error::msg("bytecode instruction offset exceeds cleanup range width"))?;
    let Some(range) = ranges.covering(offset) else {
        let metadata_present =
            !proto.failure_cleanups.is_empty() || !proto.failure_cleanup_ranges.is_empty();
        return if state.cleanup_required() && metadata_present {
            Err(Error::msg(
                "bytecode live unique state lacks failure-cleanup range",
            ))
        } else {
            Ok(())
        };
    };
    if range.start != offset {
        return Ok(());
    }
    if let Some(plan) = range.plan {
        validate_plan(proto, plan, state, true).map_err(|error| {
            Error::msg(format!(
                "failure cleanup at byte {offset} opcode {}: {error}",
                proto.code.get(host_offset).copied().unwrap_or(u8::MAX)
            ))
        })?;
    } else if state.cleanup_required() {
        return Err(Error::msg(format!(
            "bytecode live unique state has an empty failure-cleanup range at offset {offset} opcode {}",
            proto.code.get(host_offset).copied().unwrap_or(u8::MAX)
        )));
    }
    if let Some(unentered) = range.unentered_plan {
        validate_plan(
            proto,
            crate::FailureCleanupRoots::single(unentered),
            state,
            false,
        )
        .map_err(|error| {
            Error::msg(format!(
                "unentered failure cleanup at byte {offset}: {error}"
            ))
        })?;
    }
    Ok(())
}

include!("plan.rs");
