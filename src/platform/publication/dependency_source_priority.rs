//! Bounded operational priority for exact direct supplier source admission.
//! This creates no topology evidence or permission; only successful complete native/source
//! admission may populate the caller's existing checked set.
use super::*;
use crate::platform::kernel::{DependencyRecord, PackageId};
use crate::platform::package_transport::source::{CollectingStore, MAXIMUM_VALIDATION_VISITS};

pub(super) struct DependencySourcePriorityWork<'scope, 'base, S: ?Sized> {
    store: &'scope CollectingStore<'base, S>,
    checkpoint: &'scope dyn Fn() -> Result<(), Diagnostic>,
    control: &'scope ExecutionControl,
    consumed: u64,
}

impl<'scope, 'base, S: ?Sized> DependencySourcePriorityWork<'scope, 'base, S> {
    pub(super) fn new(
        store: &'scope CollectingStore<'base, S>,
        checkpoint: &'scope dyn Fn() -> Result<(), Diagnostic>,
        control: &'scope ExecutionControl,
    ) -> Self {
        Self {
            store,
            checkpoint,
            control,
            consumed: 0,
        }
    }

    fn charge(&mut self, count: u64) -> Result<(), Diagnostic> {
        self.control.check().map_err(execution_diagnostic)?;
        (self.checkpoint)()?;
        let consumed = self
            .consumed
            .checked_add(count)
            .ok_or_else(priority_limit)?;
        if self
            .store
            .visits()
            .checked_add(consumed)
            .is_none_or(|total| total > MAXIMUM_VALIDATION_VISITS)
        {
            return Err(priority_limit());
        }
        self.consumed = consumed;
        Ok(())
    }

    pub(super) fn consumed(&self) -> u64 {
        self.consumed
    }
}

fn priority_limit() -> Diagnostic {
    repository_error(
        DiagnosticClass::Resource,
        "package_source_budget",
        "candidate closure exhausted aggregate validation visits during direct source priority",
    )
}

fn lookup_work(count: usize) -> u64 {
    12 * u64::from(usize::BITS - count.max(1).leading_zeros()) + 2
}

pub(super) struct DependencySourcePriority {
    by_package: BTreeMap<PackageId, usize>,
    referenced_direct: Vec<bool>,
}

impl DependencySourcePriority {
    pub(super) fn new<S: ?Sized>(
        direct: &[DependencyRecord],
        work: &mut DependencySourcePriorityWork<'_, '_, S>,
    ) -> Result<Self, Diagnostic> {
        if direct.len() >= crate::platform::package_transport::MAXIMUM_PACKAGE_CLOSURE {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "package_revision_closure_count",
                "candidate closure exceeds the package admission including its root",
            ));
        }
        // Precharge the flag vector and bounded package/index catalogue before any growth.
        work.charge(direct.len() as u64 * (lookup_work(direct.len()) + 3))?;
        let mut by_package = BTreeMap::new();
        let referenced_direct = vec![false; direct.len()];
        for (index, binding) in direct.iter().enumerate() {
            work.charge(1)?;
            if by_package.insert(binding.package, index).is_some() {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "publication_dependency_priority_key",
                    "direct source priority received a duplicate canonical package key",
                ));
            }
        }
        Ok(Self {
            by_package,
            referenced_direct,
        })
    }

    pub(super) fn observe_dependency<S: ?Sized>(
        &mut self,
        child: &DependencyRecord,
        direct: &[DependencyRecord],
        work: &mut DependencySourcePriorityWork<'_, '_, S>,
    ) -> Result<(), Diagnostic> {
        // An exact graph/profile/package/semantic-revision/revision equality is required.
        // Package-ID coincidence alone is never coverage and grants no skip authority.
        work.charge(lookup_work(self.by_package.len()) + 5)?;
        if let Some(index) = self.by_package.get(&child.package)
            && direct[*index] == *child
        {
            self.referenced_direct[*index] = true;
        }
        Ok(())
    }

    pub(super) fn into_order<S: ?Sized>(
        self,
        work: &mut DependencySourcePriorityWork<'_, '_, S>,
    ) -> Result<Vec<usize>, Diagnostic> {
        // Preserve canonical direct order within both classes. Retain EVERY original root:
        // cycles or roots not covered by an admitted ancestor still reach native validation.
        work.charge(self.referenced_direct.len() as u64 * 3)?;
        let mut order = Vec::with_capacity(self.referenced_direct.len());
        for referenced in [false, true] {
            for (index, actual) in self.referenced_direct.iter().enumerate() {
                work.charge(1)?;
                if *actual == referenced {
                    order.push(index);
                }
            }
        }
        Ok(order)
    }
}

#[cfg(test)]
pub(super) fn record_collection(revision: crate::platform::kernel::PackageRevisionDigest) {
    tests::record_collection(revision);
}

#[cfg(test)]
#[path = "dependency_source_priority_tests.rs"]
mod tests;
