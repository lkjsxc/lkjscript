// expect: trap R0020
// pending: Automatic unwrap caller attribution and dynamic errno remain unavailable; use an explicit match and an already granted output writer. Static traps now retain the actual callee file index and line.
pub fn probe() { let failure: Result<(),i64> = Err(5i64); failure.unwrap(); }
