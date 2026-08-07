use super::StructuralError;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Ledger<T> {
    entries: Vec<T>,
}

impl<T> Ledger<T> {
    pub(super) const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, value: T) -> Result<(), StructuralError> {
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

impl<T: Eq> Ledger<T> {
    pub(super) fn push_unique(&mut self, value: T) -> Result<(), StructuralError> {
        if self.entries.contains(&value) {
            return Err(StructuralError::DuplicateDependency);
        }
        self.push(value)
    }
}
