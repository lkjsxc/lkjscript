mod common;

use common::*;
use lkjscript_resource::*;

#[test]
fn journals_merge_in_stable_task_order() -> ResourceResult<()> {
    let mut left = ResourceJournal::new(2);
    let mut right = ResourceJournal::new(2);
    right.record(ResourceJournalEntry {
        task: id(2),
        worker: WorkerId::new(1, 1),
        sequence: 0,
        category: "scratch",
        amount: 4,
    })?;
    left.record(ResourceJournalEntry {
        task: id(0),
        worker: WorkerId::new(0, 1),
        sequence: 1,
        category: "queue",
        amount: 2,
    })?;
    left.record(ResourceJournalEntry {
        task: id(0),
        worker: WorkerId::new(0, 1),
        sequence: 0,
        category: "queue",
        amount: 1,
    })?;
    let merged = ResourceJournal::merge(&[right, left], 4)?;
    assert_eq!(
        merged
            .iter()
            .map(|entry| (entry.task, entry.sequence))
            .collect::<Vec<_>>(),
        vec![(id(0), 0), (id(0), 1), (id(2), 0)]
    );
    Ok(())
}
