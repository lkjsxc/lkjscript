use super::{StructuralError, StructuralLimit};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BoundedLedger<T> {
    limit: u32,
    entries: Vec<T>,
}

impl<T> BoundedLedger<T> {
    pub(super) const fn new(limit: u32) -> Self {
        Self {
            limit,
            entries: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, value: T, kind: StructuralLimit) -> Result<(), StructuralError> {
        if self.entries.len() >= self.limit as usize {
            return Err(StructuralError::LimitExceeded(kind));
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| StructuralError::AllocationFailed)?;
        self.entries.push(value);
        Ok(())
    }

    pub(super) fn drain_reverse(&mut self) -> impl Iterator<Item = T> + '_ {
        self.entries.drain(..).rev()
    }

    pub(super) fn as_slice(&self) -> &[T] {
        &self.entries
    }

    pub(super) fn entries_mut(&mut self) -> &mut [T] {
        &mut self.entries
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }
}

impl<T: Eq> BoundedLedger<T> {
    pub(super) fn push_unique(
        &mut self,
        value: T,
        kind: StructuralLimit,
    ) -> Result<(), StructuralError> {
        if self.entries.contains(&value) {
            return Err(StructuralError::DuplicateDependency);
        }
        self.push(value, kind)
    }
}
