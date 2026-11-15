// expect: trap R0020
// pending: Report the unwrap caller's file and line plus the observed errno; a prelude line mislabeled with the consumer path does not identify the failed operation.
pub fn probe() { let failure: Result<(),i64> = Err(5i64); failure.unwrap(); }
