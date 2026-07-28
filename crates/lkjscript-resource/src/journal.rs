use crate::{ResourceError, ResourceResult, TaskId, WorkerId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceJournalEntry {
    pub task: TaskId,
    pub worker: WorkerId,
    pub sequence: u32,
    pub category: &'static str,
    pub amount: u64,
}

#[derive(Clone, Debug)]
pub struct ResourceJournal {
    limit: usize,
    entries: Vec<ResourceJournalEntry>,
}

impl ResourceJournal {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            entries: Vec::new(),
        }
    }
    pub fn record(&mut self, entry: ResourceJournalEntry) -> ResourceResult<()> {
        if self.entries.len() >= self.limit {
            return Err(ResourceError::new(
                "journal-capacity",
                "resource journal is full",
            ));
        }
        self.entries.push(entry);
        Ok(())
    }
    pub fn entries(&self) -> &[ResourceJournalEntry] {
        &self.entries
    }
    pub fn merge(journals: &[Self], limit: usize) -> ResourceResult<Vec<ResourceJournalEntry>> {
        let total = journals
            .iter()
            .map(|journal| journal.entries.len())
            .sum::<usize>();
        if total > limit {
            return Err(ResourceError::new(
                "journal-capacity",
                "merged journal exceeds limit",
            ));
        }
        let mut entries: Vec<_> = journals
            .iter()
            .flat_map(|journal| journal.entries.iter().cloned())
            .collect();
        entries.sort_by_key(|entry| (entry.task, entry.sequence, entry.worker));
        Ok(entries)
    }
}
